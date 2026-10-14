use hkdf::Hkdf;
use sha2::Sha256;
use zeroize::Zeroizing;

use super::secret_key::SECRET_KEY_BYTES;
use super::{DeviceKey, Keyring, SecretKey, cipher};
use crate::SyncError;
use crate::canonical::Canonical;
use crate::encoding::Base64;
use crate::id::ProjectId;
use crate::provider::{MemberKey, WrappedKey};

const SALT: &[u8] = b"synt.sync.wrap.v1";

pub(crate) fn wrap(
    project: &ProjectId,
    sender: &DeviceKey,
    recipient: &MemberKey,
    keyring: &Keyring,
) -> Result<WrappedKey, SyncError> {
    let ephemeral = DeviceKey::generate()?;

    let mut wrapped = WrappedKey {
        member: recipient.member.clone(),
        by: sender.member(),
        generation: keyring.generation(),
        ephemeral: ephemeral.public_key(),
        sealed: Base64::default(),
    };

    let context = context(project, &wrapped);
    let fresh = ephemeral.agree(&recipient.public_key)?;
    let lasting = sender.agree(&recipient.public_key)?;
    let key = wrapping_key(&shared(&fresh, &lasting), &context);

    wrapped.sealed = Base64::new(cipher::seal(&key, &context, &keyring.to_bytes())?);

    Ok(wrapped)
}

pub(crate) fn unwrap(
    project: &ProjectId,
    recipient: &DeviceKey,
    sender: &MemberKey,
    wrapped: &WrappedKey,
) -> Result<Keyring, SyncError> {
    sender.check()?;

    if wrapped.member != recipient.member() || wrapped.by != sender.member {
        return Err(SyncError::refused(
            "the shared key names other members than the ones it came from and to",
        ));
    }

    let context = context(project, wrapped);
    let fresh = recipient.agree(&wrapped.ephemeral)?;
    let lasting = recipient.agree(&sender.public_key)?;
    let key = wrapping_key(&shared(&fresh, &lasting), &context);

    let bytes = cipher::open(&key, &context, wrapped.sealed.get())?;
    let keyring = Keyring::from_bytes(&bytes)?;

    if keyring.generation() != wrapped.generation {
        return Err(SyncError::refused(
            "the shared key is not of the generation it claims",
        ));
    }

    Ok(keyring)
}

fn context(project: &ProjectId, wrapped: &WrappedKey) -> Vec<u8> {
    Canonical::new("synt.sync.wrap")
        .text(project.get())
        .text(wrapped.member.get())
        .text(wrapped.by.get())
        .number(u64::from(wrapped.generation.get()))
        .bytes(wrapped.ephemeral.get())
        .finish()
}

fn shared(fresh: &[u8; SECRET_KEY_BYTES], lasting: &[u8; SECRET_KEY_BYTES]) -> Zeroizing<Vec<u8>> {
    Zeroizing::new([fresh.as_slice(), lasting.as_slice()].concat())
}

fn wrapping_key(shared: &[u8], context: &[u8]) -> SecretKey {
    let mut key = Zeroizing::new([0_u8; SECRET_KEY_BYTES]);

    Hkdf::<Sha256>::new(Some(SALT), shared)
        .expand(context, key.as_mut())
        .expect("32 bytes is a valid HKDF-SHA256 output length");

    SecretKey::new(*key)
}

#[cfg(test)]
mod tests {
    use super::{unwrap, wrap};
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

        let wrapped = wrap(&project(), &alice, &bob.member_key(), &keyring).unwrap();
        let unwrapped = unwrap(&project(), &bob, &alice.member_key(), &wrapped).unwrap();

        assert_eq!(unwrapped, keyring);
    }

    #[test]
    fn nobody_else_unwraps_it() {
        let alice = DeviceKey::generate().unwrap();
        let bob = DeviceKey::generate().unwrap();
        let eve = DeviceKey::generate().unwrap();
        let keyring = Keyring::generate().unwrap();

        let wrapped = wrap(&project(), &alice, &bob.member_key(), &keyring).unwrap();

        assert!(unwrap(&project(), &eve, &alice.member_key(), &wrapped).is_err());
    }

    #[test]
    fn a_key_claimed_to_come_from_another_member_is_refused() {
        let alice = DeviceKey::generate().unwrap();
        let bob = DeviceKey::generate().unwrap();
        let mallory = DeviceKey::generate().unwrap();
        let keyring = Keyring::generate().unwrap();

        let mut wrapped = wrap(&project(), &mallory, &bob.member_key(), &keyring).unwrap();
        wrapped.by = alice.member();

        assert!(unwrap(&project(), &bob, &alice.member_key(), &wrapped).is_err());
    }

    #[test]
    fn a_key_wrapped_for_another_project_is_refused() {
        let alice = DeviceKey::generate().unwrap();
        let bob = DeviceKey::generate().unwrap();
        let keyring = Keyring::generate().unwrap();
        let other = ProjectId::parse(&"7a".repeat(16)).unwrap();

        let wrapped = wrap(&other, &alice, &bob.member_key(), &keyring).unwrap();

        assert!(unwrap(&project(), &bob, &alice.member_key(), &wrapped).is_err());
    }
}
