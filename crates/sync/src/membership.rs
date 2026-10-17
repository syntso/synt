mod standing;

pub(crate) use standing::Standing;

use std::sync::Arc;

use synt_platform::Vault;

use crate::SyncError;
use crate::id::{Generation, MemberId, ProjectId};
use crate::provider::{MemberKey, SyncProvider, WrappedKey};
use crate::report::{Access, MemberInfo};
use crate::seal::{DeviceKey, KeyWrap, Keyring};

const VAULT_PREFIX: &str = "sync.project.";

pub(crate) struct Membership {
    vault: Arc<dyn Vault>,
    provider: Arc<dyn SyncProvider>,
    project: ProjectId,
    device: DeviceKey,
}

impl Membership {
    pub(crate) fn new(
        vault: Arc<dyn Vault>,
        provider: Arc<dyn SyncProvider>,
        project: ProjectId,
    ) -> Result<Self, SyncError> {
        let device = DeviceKey::load_or_create(vault.as_ref())?;

        Ok(Self {
            vault,
            provider,
            project,
            device,
        })
    }

    pub(crate) fn me(&self) -> MemberId {
        self.device.member()
    }

    pub(crate) fn provider(&self) -> &Arc<dyn SyncProvider> {
        &self.provider
    }

    pub(crate) fn join(&self) -> Result<(), SyncError> {
        let others = self
            .provider
            .members(&self.project)?
            .into_iter()
            .any(|member| member.member != self.me());

        self.provider
            .join(&self.project, &self.device.member_key())?;

        if others || self.keyring()?.is_some() {
            return Ok(());
        }

        self.start_keyring()
    }

    pub(crate) fn forget_keyring(vault: &dyn Vault, project: &ProjectId) -> Result<(), SyncError> {
        vault.remove(&Self::vault_item(project))?;

        Ok(())
    }

    pub(crate) fn leave(&self) -> Result<(), SyncError> {
        self.provider.leave(&self.project, &self.me())
    }

    pub(crate) fn standing(&self) -> Result<Standing, SyncError> {
        let members = self.provider.members(&self.project)?;
        if !members.iter().any(|member| member.member == self.me()) {
            return Ok(Standing::Removed);
        }

        let Some(wrapped) = self.provider.key(&self.project, &self.me())? else {
            return Ok(Standing::Waiting);
        };

        let held = self.keyring()?;
        if let Some(held) = held
            .as_ref()
            .filter(|held| held.generation() >= wrapped.generation)
        {
            return Ok(Standing::Member(held.clone()));
        }

        let keyring = self.adopt(&members, &wrapped, held.as_ref())?;

        Ok(Standing::Member(keyring))
    }

    pub(crate) fn keyring_for_work(&self) -> Result<Keyring, SyncError> {
        self.standing()?.into_keyring(&self.project)
    }

    pub(crate) fn share(&self, member: &MemberId) -> Result<Generation, SyncError> {
        let keyring = self.keyring_for_work()?;
        let recipient = self.member_key(member)?;

        let wrapped = self.key_wrap().wrap(&recipient, &keyring)?;
        self.provider.share(&self.project, &[wrapped])?;

        Ok(keyring.generation())
    }

    pub(crate) fn unshare(&self, member: &MemberId) -> Result<Generation, SyncError> {
        if member == &self.me() {
            return Err(SyncError::RemovesItself);
        }

        let keyring = self.keyring_for_work()?;
        self.member_key(member)?;
        self.provider.leave(&self.project, member)?;

        let rotated = keyring.rotated()?;
        let wrapped = self.wrap_for_members(&rotated)?;
        self.provider.share(&self.project, &wrapped)?;
        self.save(&rotated)?;

        Ok(rotated.generation())
    }

    pub(crate) fn members(&self) -> Result<Vec<MemberInfo>, SyncError> {
        let me = self.me();
        let mut members = Vec::new();

        for member in self.provider.members(&self.project)? {
            let shared = self.provider.key(&self.project, &member.member)?.is_some();
            let access = if shared {
                Access::Member
            } else {
                Access::Waiting
            };

            members.push(MemberInfo {
                this_device: member.member == me,
                access,
                member: member.member,
            });
        }

        Ok(members)
    }

    fn start_keyring(&self) -> Result<(), SyncError> {
        let keyring = Keyring::generate()?;
        let wrapped = self.key_wrap().wrap(&self.device.member_key(), &keyring)?;

        self.save(&keyring)?;
        self.provider.share(&self.project, &[wrapped])
    }

    fn adopt(
        &self,
        members: &[MemberKey],
        wrapped: &WrappedKey,
        held: Option<&Keyring>,
    ) -> Result<Keyring, SyncError> {
        let sender = members
            .iter()
            .find(|member| member.member == wrapped.by)
            .ok_or_else(|| SyncError::SharedByStranger(wrapped.by.clone()))?;

        let keyring = self.key_wrap().unwrap(sender, wrapped)?;

        if held.is_some_and(|held| !keyring.keeps(held)) {
            return Err(SyncError::KeysDropped);
        }

        self.save(&keyring)?;

        Ok(keyring)
    }

    fn wrap_for_members(&self, keyring: &Keyring) -> Result<Vec<WrappedKey>, SyncError> {
        let mut wrapped = Vec::new();

        for member in self.provider.members(&self.project)? {
            if self.provider.key(&self.project, &member.member)?.is_none() {
                continue;
            }

            wrapped.push(self.key_wrap().wrap(&member, keyring)?);
        }

        Ok(wrapped)
    }

    fn member_key(&self, member: &MemberId) -> Result<MemberKey, SyncError> {
        self.provider
            .members(&self.project)?
            .into_iter()
            .find(|key| &key.member == member)
            .ok_or_else(|| SyncError::NotJoined {
                member: member.clone(),
                project: self.project.clone(),
            })
    }

    fn keyring(&self) -> Result<Option<Keyring>, SyncError> {
        self.vault
            .read(&Self::vault_item(&self.project))?
            .map(|bytes| Keyring::from_bytes(&bytes))
            .transpose()
    }

    fn save(&self, keyring: &Keyring) -> Result<(), SyncError> {
        self.vault
            .write(&Self::vault_item(&self.project), &keyring.to_bytes())?;

        Ok(())
    }

    fn key_wrap(&self) -> KeyWrap<'_> {
        KeyWrap::new(&self.project, &self.device)
    }

    fn vault_item(project: &ProjectId) -> String {
        format!("{VAULT_PREFIX}{project}")
    }
}
