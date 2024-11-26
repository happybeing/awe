/*
Copyright (c) 2024-2025 Mark Hughes

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU Affero General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
GNU Affero General Public License for more details.

You should have received a copy of the GNU Affero General Public License
along with this program. If not, see <https://www.gnu.org/licenses/>.
*/

pub mod file_tree;

use std::marker::PhantomData;

use bytes::Bytes;
use color_eyre::eyre::{eyre, Error, Result};
use serde::{de::DeserializeOwned, Serialize};
use xor_name::XorName;

use autonomi::client::registers::{Register, RegisterSecretKey};
use autonomi::client::Client;
use autonomi::Wallet;
use sn_registers::RegisterAddress;

use crate::dweb::autonomi::access::keys::get_register_signing_key;
use crate::dweb::data::autonomi_get_file;

const LARGEST_VERSION: u64 = 9007199254740991; // JavaScript Number.MAX_SAFE_INTEGER

/// The Trove trait enables any serializable struct to be saved in Autonomi
/// decentralised storage using a TroveHistory<T> for your struct. The TroveHistory then
/// gives access to every version of the struct that has ever been stored on Autonomi.
///
/// For example, using the built-in dweb::trove::FileTree struct you can store and access
/// every published version of a website or tree of files.
///
/// Notes:
/// - the dweb-cli supports viewing of versioned websites and directories using
/// a standard web browser, including viewing every version published on Autonomi (similar
/// to the Internet Archive).
/// -  TroveHistory is a CRDT based ordered collection of stored Trove structs,
/// amounting to a versioned history for any piece or set of data which
/// implements  Trove trait.
pub trait Trove {
    fn trove_type() -> XorName;
}

/// A CRDT based ordered collection of Trove items of the same type. A
/// TroveHistory provides a versioned history for a Trove, which might be
/// a file, a collection of files such as a directory, or all the files
/// and settings that make up a website and so on.
pub struct TroveHistory<T: Trove + Serialize + DeserializeOwned> {
    // For operations when no version is specified. Typically, None implies most recent
    default_version: Option<u64>,
    // Cached data for the selected version
    cached_version: Option<TroveVersion<T>>,

    // owner_secret is only required for publish/update (not access)
    owner_secret: Option<RegisterSecretKey>,
    register: Register,

    // Pretend we hold a Trove so we can restrict some values to type T in the implementation
    phantom: std::marker::PhantomData<T>,
}

impl<T: Trove + Serialize + DeserializeOwned> TroveHistory<T> {
    /// Gets an existing Register or creates a new register online
    /// The owner_secret is required when creating and for adding entries (publish/update)
    pub async fn new(
        client: &Client,
        address: Option<RegisterAddress>,
        owner_secret: Option<RegisterSecretKey>,
        wallet: &Wallet,
    ) -> Result<Self> {
        let mut register_signing_key = owner_secret;
        let register = if let Some(addr) = address {
            client.register_get(addr).await
        } else {
            let signing_key = if register_signing_key.is_some() {
                register_signing_key.unwrap()
            } else {
                println!(
                    "Register signing key was not provided so attempting to obtain from system"
                );
                match get_register_signing_key() {
                    Ok(signing_key) => signing_key,
                    Err(e) => {
                        println!("Failed to get register signing key: {e}");
                        return Err(e);
                    }
                }
            };
            register_signing_key = Some(signing_key.clone());

            // let secret_key =
            //     get_secret_key_from_env().expect("SECRET_KEY environment variable not set");
            // let secret_key = autonomi::client::registers::RegisterSecretKey::from_hex(&secret_key)
            //     .expect("Failed to decode SECRET_KEY environment variable");

            let mut rng = rand::thread_rng();
            let name = format!("{:64x}", XorName::random(&mut rng));
            client
                .register_create(None, name.as_str(), signing_key, wallet)
                .await
        };

        if register.is_ok() {
            let mut history = TroveHistory {
                default_version: None,
                cached_version: None,
                register: register.unwrap(),
                owner_secret: register_signing_key,
                phantom: PhantomData,
            };

            // Write the trove_type to the first entry
            history
                .add_xor_name(&client, &Self::trove_type(), &wallet)
                .await?;
            history.update_default_version();
            Ok(history)
        } else {
            Err(register.unwrap_err().into())
        }
    }

    /// The owner_secret is only required for publish/update using the returned TroveHistory (not access)
    pub fn from_client_register(
        client_register: Register,
        owner_secret: Option<RegisterSecretKey>,
    ) -> TroveHistory<T> {
        let mut history = TroveHistory::<T> {
            default_version: None,
            cached_version: None,
            register: client_register,
            owner_secret: owner_secret.clone(),
            phantom: PhantomData,
        };
        history.update_default_version();
        history
    }

    /// Load a Register from the network and return wrapped in TroveHistory
    /// The owner_secret is only required for publish/update using the returned TroveHistory (not access)
    pub async fn from_register_address(
        register_address: RegisterAddress,
        client: &Client,
        owner_secret: Option<RegisterSecretKey>,
    ) -> Result<TroveHistory<T>> {
        // Check it exists to avoid accidental creation (and payment)
        let result = client.register_get(register_address).await;
        let mut history = if result.is_ok() {
            TroveHistory::<T>::from_client_register(result.unwrap(), owner_secret)
        } else {
            println!("DEBUG: from_register_address() error:");
            return Err(eyre!("register not found on network"));
        };
        history.update_default_version();
        Ok(history)
    }

    fn owner_secret(&self) -> Result<RegisterSecretKey, Error> {
        match self.owner_secret.clone() {
            Some(owner_secret) => Ok(owner_secret),
            None => Err(eyre!(
                "ERROR: TroveHistory can't update register without ::owner_secret"
            )),
        }
    }

    fn update_default_version(&mut self) -> Option<u64> {
        self.default_version = match self.num_versions() {
            Ok(version) => Some(version),
            Err(_) => None,
        };
        self.default_version
    }

    pub fn register_address(&self) -> &RegisterAddress {
        self.register.address()
    }

    fn trove_type() -> XorName {
        T::trove_type()
    }

    /// Return the number of entries in the register
    /// This is one more than the number of versions
    /// because the first entry is reserved for use
    /// as a type (which may point to metadata about
    /// the type). Example types include file system
    /// and website.
    pub fn num_entries(&self) -> u64 {
        crate::commands::helpers::node_entries_as_vec(&self.register).len() as u64
    }

    /// Return the number of available versions
    /// or an error if no versions are available.
    /// The first version is 1 last version is num_versions()
    pub fn num_versions(&self) -> Result<u64> {
        let num_entries = self.num_entries();

        if num_entries == 0 {
            let message = "register is empty (0 entries)";
            Err(eyre!(message))
        } else {
            Ok(num_entries - 1)
        }
    }

    /// Download a FileTree from the network
    async fn trove_download(&mut self, client: &Client, data_address: XorName) -> Result<T> {
        println!("DEBUG file_tree_download() at {data_address:64x}");
        match autonomi_get_file(data_address, client).await {
            Ok(content) => {
                println!("Retrieved {} bytes", content.len());
                let metadata: T = match rmp_serde::from_slice(&content) {
                    Ok(metadata) => metadata,
                    Err(e) => {
                        println!("FAILED: {e}");
                        return Err(eyre!(e));
                    }
                };
                Ok(metadata)
            }

            Err(e) => {
                println!("FAILED: {e}");
                Err(eyre!(e))
            }
        }
    }

    /// Get the metadata entry for a given version.
    /// The first entry in the register is version 0, but that is reserved so the
    /// first version of a website is 1 and the last is the number of entries - 1
    pub fn get_version_entry(&self, version: u64) -> Result<XorName> {
        println!("DEBUG XXXXXX get_version_entry(version: {version})");
        let entries_vec = crate::commands::helpers::node_entries_as_vec(&self.register);
        let num_entries = entries_vec.len();

        // This is used to hold a value for use by the Svelte frontend
        let max_version = if num_entries > 0 {
            num_entries as u64 - 1
        } else {
            0
        };

        // set_version_max(max_version as u64); // TODO awe needs to set this explicitly

        // Note the first node is a marker, and not used so max version is length - 1
        if version <= max_version {
            let entry = &entries_vec[version as usize];
            Ok(crate::commands::helpers::xorname_from_entry(&entry))
        } else {
            Err(eyre!(
                "Version {version} too large. Maximum is {}",
                max_version
            ))
        }
    }

    // Returns the version of the cached entry if present
    pub fn get_cached_version(&self) -> Option<u64> {
        if let Some(site) = &self.cached_version {
            if site.metadata.is_some() {
                return Some(site.version);
            }
        }
        None
    }

    /// Adds an XorName to the register, merging any branches
    pub async fn add_xor_name(
        &mut self,
        client: &Client,
        xor_value: &XorName,
        _wallet: &Wallet, // Include for when updates are charged for
    ) -> Result<()> {
        let register_xor_address = self.register.address().to_hex();
        println!("Updating register    : {register_xor_address}");
        // The first register_get() has been added for testing (as reg_update() isn't always changing some registers)
        match client.register_get(self.register.address().clone()).await {
            Ok(register) => {
                let owner_secret = if self.owner_secret.is_some() {
                    self.owner_secret.clone().unwrap()
                } else {
                    return Err(eyre!(
                        "Cannot update Register - register secret key is None"
                    ));
                };
                let register_xor_address = register.address().to_hex();
                println!("Register get returned: {register_xor_address}");
                let values = self.register.values();
                println!("Before register_update()...do client.register_get()...");
                println!("      Register has {} values", values.len());
                println!("      Register has {} entries", self.num_entries());
                let merkle_reg = self.register.inner_merkle_reg();
                println!("      Register {merkle_reg:?}");

                println!("Calling register_update() with value: {xor_value}");
                match client
                    .register_update(
                        self.register.clone(),
                        Bytes::from(xor_value.to_vec()),
                        owner_secret.clone(),
                    )
                    .await
                {
                    Ok(_) => {
                        let values = self.register.values();
                        println!("After update...");
                        println!("      Register has {} values", values.len());
                        println!("      Register has {} entries", self.num_entries());
                        let merkle_reg = self.register.inner_merkle_reg();
                        println!("      Register {merkle_reg:?}");

                        // It is necessary to get the register from the network to have it's entries accessible
                        self.register = match client
                            .register_get(self.register.address().clone())
                            .await
                        {
                            Ok(register) => {
                                let values = self.register.values();
                                println!("After update...and get...");
                                println!("      Register has {} values", values.len());
                                println!("      Register has {} entries", self.num_entries());
                                let merkle_reg = self.register.inner_merkle_reg();
                                println!("      Register {merkle_reg:?}");

                                let register_xor_address = self.register.address().to_hex();
                                println!("client.register_update() added entry to register: {register_xor_address}");
                                self.update_default_version();
                                register
                            }
                            Err(e) => {
                                return Err(eyre!(
                                    "DEBUG failed to get register that was just updated!\n{e}"
                                ))
                            }
                        };

                        let register_xor_address = self.register.address().to_hex();
                        println!(
                            "DEBUG client.register_update() added entry to register: {register_xor_address}"
                        );
                        let merkle_reg = self.register.inner_merkle_reg();
                        println!("DEBUG register.inner_merkle_reg():\n{merkle_reg:?}");
                    }
                    Err(e) => {
                        return Err(eyre!("Failed to add XorName to register: {e:?}"));
                    }
                }
            }
            Err(e) => return Err(eyre!("DEBUG failed to get register prior to update!\n{e}")),
        };

        Ok(())
    }

    /// Publishes a new version pointing to the metadata provided
    /// which becomes the newly selected version
    /// Returns the selected version as a number
    pub async fn publish_new_version(
        &mut self,
        client: &Client,
        metadata_address: &XorName,
        wallet: &Wallet,
    ) -> Result<u64> {
        self.add_xor_name(client, metadata_address, wallet).await?;
        println!("metadata_address added to register: {metadata_address:64x}");
        let version = self.num_versions()?;
        self.cached_version = Some(TroveVersion::<T>::new(
            version,
            metadata_address.clone(),
            None,
        ));
        Ok(version)
    }

    /// Makes the given version current by retrieving and storing the Trove.
    /// If version is None, selects the most recent version.
    /// The first version is 1, and the last version is WebsiteVersions::num_versions()
    /// If version 0 or None is specified, the default/last version will be retrieved.
    /// After success, the WebsiteMetadata can be accessed using current metadata.
    /// If it fails, the selected version will be unchanged and any cached data retained.
    // Version 0 is hidden (and used to id WebsiteMetadata) but can be accessed by
    // specifying a version of LARGEST_VERSION
    async fn fetch_version_metadata(
        &mut self,
        client: &Client,
        version: Option<u64>,
    ) -> Result<u64> {
        println!(
            "DEBUG fetch_version_metadata() self.cached_version.is_some(): {}",
            self.cached_version.is_some()
        );
        let mut version = if version.is_some() {
            version.unwrap()
        } else {
            0
        };

        if version == 0 {
            if self.default_version.is_some() {
                version = self.default_version.unwrap()
            } else {
                println!("No default_version available to select");
                return Err(eyre!("No default_version available to select"));
            }
        };

        // Allow access to the zeroth version
        let version = if version == LARGEST_VERSION {
            0
        } else {
            version
        };

        // Return if already cached
        if let Some(site) = &self.cached_version {
            if site.version == version && site.metadata.is_some() {
                return Ok(version);
            }
        }

        let data_address = match self.get_metadata_address_from_register(version).await {
            Ok(data_address) => data_address,
            Err(e) => {
                println!("select_version() failed to get version {version} from register");
                return Err(eyre!(e));
            }
        };

        let metadata: Option<T> = match self.trove_download(client, data_address).await {
            Ok(metadata) => Some(metadata),
            Err(e) => {
                println!("select_version() failed to get website metadata from network: {e}");
                None
            }
        };

        self.cached_version = Some(TroveVersion::new(version, data_address, metadata));
        Ok(version)
    }

    // Operations which will be applied to the currently selected version
    pub async fn get_metadata_address_from_register(&self, version: u64) -> Result<XorName> {
        println!("DEBUG XXXXXX get_metadata_address_from_register(version: {version})");
        // Use cached site value if available
        if let Some(site) = &self.cached_version {
            if site.version == version {
                println!("DEBUG XXXXXX get_metadata_address_from_register() returning cached metadata address: {}", &site.metadata_address);
                return Ok(site.metadata_address.clone());
            }
        };
        self.get_version_entry(version)
    }
}

/// The state of a Trove struct at a given version  with optional cache of its data
struct TroveVersion<ST: Trove + Serialize + DeserializeOwned> {
    // Version of Some(metadata) with address metadata_address
    version: u64,

    metadata_address: XorName,
    metadata: Option<ST>,
}

impl<ST: Trove + Serialize + DeserializeOwned> TroveVersion<ST> {
    pub fn new(version: u64, metadata_address: XorName, metadata: Option<ST>) -> TroveVersion<ST> {
        TroveVersion {
            version,
            metadata_address: metadata_address,
            metadata,
        }
    }
}
