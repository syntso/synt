use std::sync::Arc;

use synt_platform::Vault;

use super::Standing;
use crate::SyncError;
use crate::id::{Generation, MemberId, ProjectId};
use crate::provider::{MemberKey, SyncProvider, WrappedKey};
use crate::report::{Access, MemberInfo};
use crate::seal::{DeviceKey, Keyring, unwrap, wrap};

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
        vault.remove(&vault_item(project)).map_err(SyncError::from)
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
        match self.standing()? {
            Standing::Member(keyring) => Ok(keyring),
            Standing::Waiting => Err(SyncError::NoKey(self.project.to_string())),
            Standing::Removed => Err(SyncError::Removed(self.project.to_string())),
        }
    }

    pub(crate) fn share(&self, member: &MemberId) -> Result<Generation, SyncError> {
        let keyring = self.keyring_for_work()?;
        let recipient = self.member_key(member)?;

        let wrapped = wrap(&self.project, &self.device, &recipient, &keyring)?;
        self.provider.share(&self.project, &[wrapped])?;

        Ok(keyring.generation())
    }

    pub(crate) fn unshare(&self, member: &MemberId) -> Result<Generation, SyncError> {
        if member == &self.me() {
            return Err(SyncError::invalid(
                "a device cannot remove itself; use sync.disable on it",
            ));
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

            members.push(MemberInfo {
                this_device: member.member == me,
                access: if shared {
                    Access::Member
                } else {
                    Access::Waiting
                },
                member: member.member,
            });
        }

        Ok(members)
    }

    fn start_keyring(&self) -> Result<(), SyncError> {
        let keyring = Keyring::generate()?;
        let wrapped = wrap(
            &self.project,
            &self.device,
            &self.device.member_key(),
            &keyring,
        )?;

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
            .ok_or_else(|| {
                SyncError::refused(format!(
                    "the project key was shared by {}, who is not a member",
                    wrapped.by
                ))
            })?;

        let keyring = unwrap(&self.project, &self.device, sender, wrapped)?;

        if held.is_some_and(|held| !keyring.keeps(held)) {
            return Err(SyncError::refused(
                "the new project key drops keys this device already holds",
            ));
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

            wrapped.push(wrap(&self.project, &self.device, &member, keyring)?);
        }

        Ok(wrapped)
    }

    fn member_key(&self, member: &MemberId) -> Result<MemberKey, SyncError> {
        self.provider
            .members(&self.project)?
            .into_iter()
            .find(|key| &key.member == member)
            .ok_or_else(|| {
                SyncError::invalid(format!(
                    "{member} has not joined project {}; run sync.enable on that device first",
                    self.project
                ))
            })
    }

    fn keyring(&self) -> Result<Option<Keyring>, SyncError> {
        self.vault
            .read(&self.vault_item())?
            .map(|bytes| Keyring::from_bytes(&bytes))
            .transpose()
    }

    fn save(&self, keyring: &Keyring) -> Result<(), SyncError> {
        self.vault
            .write(&self.vault_item(), &keyring.to_bytes())
            .map_err(SyncError::from)
    }

    fn vault_item(&self) -> String {
        vault_item(&self.project)
    }
}

fn vault_item(project: &ProjectId) -> String {
    format!("{VAULT_PREFIX}{project}")
}
