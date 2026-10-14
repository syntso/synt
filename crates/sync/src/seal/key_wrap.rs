use zeroize::Zeroizing;

use super::{DeviceKey, Keyring, SecretKey};
use crate::SyncError;
use crate::canonical::Canonical;
use crate::encoding::Base64;
use crate::id::ProjectId;
use crate::provider::{MemberKey, WrappedKey};

const SALT: &[u8] = b"synt.sync.wrap.v1";

pub(crate) struct KeyWrap<'a> {
    project: &'a ProjectId,
    device: &'a DeviceKey,
}

impl<'a> KeyWrap<'a> {
    pub(crate) fn new(project: &'a ProjectId, device: &'a DeviceKey) -> Self {
        Self { project, device }
    }

    pub(crate) fn wrap(
        &self,
        recipient: &MemberKey,
        keyring: &Keyring,
    ) -> Result<WrappedKey, SyncError> {
        let ephemeral = DeviceKey::generate()?;

        let mut wrapped = WrappedKey {
            member: recipient.member.clone(),
            by: self.device.member(),
            generation: keyring.generation(),
            ephemeral: ephemeral.public_key(),
            sealed: Base64::default(),
        };

        let context = self.context(&wrapped);
        let fresh = ephemeral.agree(&recipient.public_key)?;
        let lasting = self.device.agree(&recipient.public_key)?;
        let key = Self::wrapping_key(&fresh, &lasting, &context)?;

        wrapped.sealed = Base64::new(key.seal(&context, &keyring.to_bytes())?);

        Ok(wrapped)
    }

    pub(crate) fn unwrap(
        &self,
        sender: &MemberKey,
        wrapped: &WrappedKey,
    ) -> Result<Keyring, SyncError> {
        sender.check()?;

        if wrapped.member != self.device.member() || wrapped.by != sender.member {
            return Err(SyncError::refused(
                "the shared key names other members than the ones it came from and to",
            ));
        }

        let context = self.context(wrapped);
        let fresh = self.device.agree(&wrapped.ephemeral)?;
        let lasting = self.device.agree(&sender.public_key)?;
        let key = Self::wrapping_key(&fresh, &lasting, &context)?;

        let keyring = Keyring::from_bytes(&key.open(&context, wrapped.sealed.get())?)?;

        if keyring.generation() != wrapped.generation {
            return Err(SyncError::refused(
                "the shared key is not of the generation it claims",
            ));
        }

        Ok(keyring)
    }

    fn context(&self, wrapped: &WrappedKey) -> Vec<u8> {
        Canonical::new("synt.sync.wrap")
            .text(self.project.get())
            .text(wrapped.member.get())
            .text(wrapped.by.get())
            .number(u64::from(wrapped.generation.get()))
            .bytes(wrapped.ephemeral.get())
            .finish()
    }

    fn wrapping_key(
        fresh: &[u8; SecretKey::BYTES],
        lasting: &[u8; SecretKey::BYTES],
        context: &[u8],
    ) -> Result<SecretKey, SyncError> {
        let shared = Zeroizing::new([fresh.as_slice(), lasting.as_slice()].concat());

        SecretKey::derive(SALT, &shared, context)
    }
}

#[cfg(test)]
mod tests {
    use super::KeyWrap;
    use crate::id::ProjectId;
    use crate::seal::{DeviceKey, Keyring};

    fn project() -> ProjectId {
        ProjectId::parse(&"4f".repeat(16)).unwrap()
    }

    #[test]
    fn the_recipient_unwraps_the_keyring_the_sender_wrapped() {
        let alice = DeviceKey::generate().unwrap();
        let bob = DeviceKey::generate().unwrap();
        let keyring = Keyring::generate().unwrap().rotated().unwrap();
        let project = project();

        let wrapped = KeyWrap::new(&project, &alice)
            .wrap(&bob.member_key(), &keyring)
            .unwrap();
        let unwrapped = KeyWrap::new(&project, &bob)
            .unwrap(&alice.member_key(), &wrapped)
            .unwrap();

        assert_eq!(unwrapped, keyring);
    }

    #[test]
    fn nobody_else_unwraps_it() {
        let alice = DeviceKey::generate().unwrap();
        let bob = DeviceKey::generate().unwrap();
        let eve = DeviceKey::generate().unwrap();
        let keyring = Keyring::generate().unwrap();
        let project = project();

        let wrapped = KeyWrap::new(&project, &alice)
            .wrap(&bob.member_key(), &keyring)
            .unwrap();

        assert!(
            KeyWrap::new(&project, &eve)
                .unwrap(&alice.member_key(), &wrapped)
                .is_err()
        );
    }

    #[test]
    fn a_key_claimed_to_come_from_another_member_is_refused() {
        let alice = DeviceKey::generate().unwrap();
        let bob = DeviceKey::generate().unwrap();
        let mallory = DeviceKey::generate().unwrap();
        let keyring = Keyring::generate().unwrap();
        let project = project();

        let mut wrapped = KeyWrap::new(&project, &mallory)
            .wrap(&bob.member_key(), &keyring)
            .unwrap();
        wrapped.by = alice.member();

        assert!(
            KeyWrap::new(&project, &bob)
                .unwrap(&alice.member_key(), &wrapped)
                .is_err()
        );
    }

    #[test]
    fn a_key_wrapped_for_another_project_is_refused() {
        let alice = DeviceKey::generate().unwrap();
        let bob = DeviceKey::generate().unwrap();
        let keyring = Keyring::generate().unwrap();
        let other = ProjectId::parse(&"7a".repeat(16)).unwrap();
        let project = project();

        let wrapped = KeyWrap::new(&other, &alice)
            .wrap(&bob.member_key(), &keyring)
            .unwrap();

        assert!(
            KeyWrap::new(&project, &bob)
                .unwrap(&alice.member_key(), &wrapped)
                .is_err()
        );
    }
}
