//! ArtiEphemeralKeystore implementation (in-memory ephemeral key storage)

pub(crate) mod err;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use tor_error::internal;
use tor_key_forge::{EncodableItem, ErasedKey, KeystoreItem, KeystoreItemType};

use crate::keystore::ephemeral::err::ArtiEphemeralKeystoreError;
use crate::Error;
use crate::{ArtiPath, KeyPath, KeySpecifier, Keystore, KeystoreId};

/// The identifier of a key stored in the `ArtiEphemeralKeystore`.
type KeyIdent = (ArtiPath, KeystoreItemType);

/// The Ephemeral Arti key store
///
/// This is a purely in-memory key store. Keys written to this store
/// are never written to disk, and are stored in-memory as [`KeystoreItem`]s.
/// Keys saved in this Keystore do not persist between restarts!
///
/// While Arti never writes the keys for this key store to disk, the operating
/// system may do so for reasons outside of this library's control. Some
/// examples are swapping RAM to disk, generating core dumps, invoking
/// suspend-to-disk power management, etc. This key store does not attempt to
/// prevent this operating system behaviour.
pub struct ArtiEphemeralKeystore {
    /// Identifier hard-coded to 'ephemeral'
    id: KeystoreId,
    /// Keys stored as [`KeystoreItem`].
    key_dictionary: Arc<Mutex<HashMap<KeyIdent, KeystoreItem>>>,
}

impl ArtiEphemeralKeystore {
    /// Create a new [`ArtiEphemeralKeystore`]
    pub fn new(id: String) -> Self {
        Self {
            id: KeystoreId(id),
            key_dictionary: Default::default(),
        }
    }
}

impl Keystore for ArtiEphemeralKeystore {
    fn id(&self) -> &KeystoreId {
        &self.id
    }

    fn contains(
        &self,
        key_spec: &dyn KeySpecifier,
        item_type: &KeystoreItemType,
    ) -> Result<bool, Error> {
        let arti_path = key_spec
            .arti_path()
            .map_err(ArtiEphemeralKeystoreError::ArtiPathUnavailableError)?;
        let key_dictionary = self.key_dictionary.lock().expect("lock poisoned");
        let contains_key = key_dictionary.contains_key(&(arti_path, item_type.clone()));
        Ok(contains_key)
    }

    fn get(
        &self,
        key_spec: &dyn KeySpecifier,
        item_type: &KeystoreItemType,
    ) -> Result<Option<ErasedKey>, Error> {
        let arti_path = key_spec
            .arti_path()
            .map_err(ArtiEphemeralKeystoreError::ArtiPathUnavailableError)?;
        let key_dictionary = self.key_dictionary.lock().expect("lock poisoned");
        match key_dictionary.get(&(arti_path.clone(), item_type.clone())) {
            Some(key) => {
                let key: KeystoreItem = key.clone();
                let key: ErasedKey = key.into_erased()?;
                Ok(Some(key))
            }
            None => Ok(None),
        }
    }

    fn insert(
        &self,
        key: &dyn EncodableItem,
        key_spec: &dyn KeySpecifier,
        item_type: &KeystoreItemType,
    ) -> Result<(), Error> {
        let arti_path = key_spec
            .arti_path()
            .map_err(ArtiEphemeralKeystoreError::ArtiPathUnavailableError)?;
        let key_data = key.as_keystore_item()?;

        // TODO: add item_type validation to Keystore::get and Keystore::remove.
        // The presence of a key with a mismatched item_type can be either due to keystore
        // corruption, or API misuse. We will need a new error type and corresponding ErrorKind for
        // that).
        //
        // TODO: add item_type validation to ArtiNativeKeystore
        if key_data.item_type()? != *item_type {
            // This can never happen unless:
            //   * Keystore::insert is called directly with an incorrect KeystoreItemType for `key`, or
            //   * Keystore::insert is called via KeyMgr, but the EncodableItem implementation of
            //   the key is broken. EncodableItem can't be implemented by external types,
            //   so a broken implementation means we have an internal bug.
            return Err(internal!(
                "the specified KeystoreItemType does not match key type of the inserted key?!"
            )
            .into());
        }

        // save to dictionary
        let mut key_dictionary = self.key_dictionary.lock().expect("lock poisoned");
        let _ = key_dictionary.insert((arti_path, item_type.clone()), key_data);
        Ok(())
    }

    fn remove(
        &self,
        key_spec: &dyn KeySpecifier,
        item_type: &KeystoreItemType,
    ) -> Result<Option<()>, Error> {
        let arti_path = key_spec
            .arti_path()
            .map_err(ArtiEphemeralKeystoreError::ArtiPathUnavailableError)?;
        let mut key_dictionary = self.key_dictionary.lock().expect("lock poisoned");
        Ok(key_dictionary
            .remove(&(arti_path, item_type.clone()))
            .map(|_| ()))
    }

    fn list(&self) -> Result<Vec<(KeyPath, KeystoreItemType)>, Error> {
        let key_dictionary = self.key_dictionary.lock().expect("lock poisoned");
        Ok(key_dictionary
            .keys()
            .map(|(arti_path, item_type)| (arti_path.clone().into(), item_type.clone()))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    // @@ begin test lint list maintained by maint/add_warning @@
    #![allow(clippy::bool_assert_comparison)]
    #![allow(clippy::clone_on_copy)]
    #![allow(clippy::dbg_macro)]
    #![allow(clippy::mixed_attributes_style)]
    #![allow(clippy::print_stderr)]
    #![allow(clippy::print_stdout)]
    #![allow(clippy::single_char_pattern)]
    #![allow(clippy::unwrap_used)]
    #![allow(clippy::unchecked_duration_subtraction)]
    #![allow(clippy::useless_vec)]
    #![allow(clippy::needless_pass_by_value)]
    //! <!-- @@ end test lint list maintained by maint/add_warning @@ -->

    use tor_basic_utils::test_rng::testing_rng;
    use tor_key_forge::KeyType;
    use tor_llcrypto::pk::ed25519;

    use super::*;

    use crate::test_utils::TestSpecifier;

    // some helper methods

    fn key() -> Box<dyn EncodableItem> {
        let mut rng = testing_rng();
        let keypair = ed25519::Keypair::generate(&mut rng);
        Box::new(keypair)
    }

    fn key_type() -> KeystoreItemType {
        KeyType::Ed25519Keypair.into()
    }

    fn key_type_bad() -> KeystoreItemType {
        KeyType::X25519StaticKeypair.into()
    }

    fn key_spec() -> Box<dyn KeySpecifier> {
        Box::<TestSpecifier>::default()
    }

    // tests!

    #[test]
    fn id() {
        let key_store = ArtiEphemeralKeystore::new("test-ephemeral".to_string());

        assert_eq!(&KeystoreId("test-ephemeral".to_string()), key_store.id());
    }

    #[test]
    fn contains() {
        let key_store = ArtiEphemeralKeystore::new("test-ephemeral".to_string());

        // verify no key in store
        assert!(!key_store
            .contains(key_spec().as_ref(), &key_type())
            .unwrap());

        // insert key and verify in store
        assert!(key_store
            .insert(key().as_ref(), key_spec().as_ref(), &key_type())
            .is_ok());
        assert!(key_store
            .contains(key_spec().as_ref(), &key_type())
            .unwrap());
    }

    #[test]
    fn get() {
        let key_store = ArtiEphemeralKeystore::new("test-ephemeral".to_string());

        // verify no result to get
        assert!(key_store
            .get(key_spec().as_ref(), &key_type())
            .unwrap()
            .is_none());

        // insert and verify get is a result
        assert!(key_store
            .insert(key().as_ref(), key_spec().as_ref(), &key_type())
            .is_ok());

        let key = key_store
            .get(key_spec().as_ref(), &key_type())
            .unwrap()
            .unwrap();

        // Ensure the returned key is of the right type
        assert!(key.downcast::<ed25519::Keypair>().is_ok());
    }

    #[test]
    fn insert() {
        let key_store = ArtiEphemeralKeystore::new("test-ephemeral".to_string());

        // verify inserting a key with the wrong key type fails
        assert!(key_store
            .insert(key().as_ref(), key_spec().as_ref(), &key_type_bad())
            .is_err());
        // further ensure there are no side effects
        assert!(!key_store
            .contains(key_spec().as_ref(), &key_type_bad())
            .unwrap());
        assert!(key_store
            .get(key_spec().as_ref(), &key_type_bad())
            .unwrap()
            .is_none());
        assert!(key_store.list().unwrap().is_empty());

        // verify inserting a good key succeeds
        assert!(key_store
            .insert(key().as_ref(), key_spec().as_ref(), &key_type())
            .is_ok());

        // further ensure correct side effects
        assert!(key_store
            .contains(key_spec().as_ref(), &key_type())
            .unwrap());
        assert!(key_store
            .get(key_spec().as_ref(), &key_type())
            .unwrap()
            .is_some());
        assert_eq!(key_store.list().unwrap().len(), 1);
    }

    #[test]
    fn remove() {
        let key_store = ArtiEphemeralKeystore::new("test-ephemeral".to_string());

        // verify removing from an empty store returns None
        assert!(key_store
            .remove(key_spec().as_ref(), &key_type())
            .unwrap()
            .is_none());

        // verify inserting and removing results in Some(())
        assert!(key_store
            .insert(key().as_ref(), key_spec().as_ref(), &key_type())
            .is_ok());
        assert!(key_store
            .remove(key_spec().as_ref(), &key_type())
            .unwrap()
            .is_some());
    }

    #[test]
    fn list() {
        let key_store = ArtiEphemeralKeystore::new("test-ephemeral".to_string());

        // verify empty by default
        assert!(key_store.list().unwrap().is_empty());

        // verify size 1 after inserting a key
        assert!(key_store
            .insert(key().as_ref(), key_spec().as_ref(), &key_type())
            .is_ok());
        assert_eq!(key_store.list().unwrap().len(), 1);
    }
}
