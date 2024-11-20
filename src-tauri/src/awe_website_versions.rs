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
use bytes::Bytes;
use color_eyre::eyre::{eyre, Error, Result};
use tauri::http::status::StatusCode;
use xor_name::XorName;

use crate::helpers::autonomi::access::keys::get_register_signing_key;

use autonomi::client::registers::{Register, RegisterSecretKey};
use autonomi::client::Client;
use autonomi::Wallet;
use sn_registers::RegisterAddress;
// use sn_registers::RegisterCrdt; // Wraps MerkleReg
// use autonomi::client::registers::Register; // Wraps RegisterCrdt
// use sn_registers::Register as SnRegister;  // RegisterAddress and Permissions

#[cfg(not(feature = "skip-network-compatibility-check"))]
use crate::awe_client::str_to_xor_name;
use crate::awe_protocols::{set_version_loaded, set_version_max};
use crate::awe_website_metadata::{get_website_metadata_from_network, WebsiteMetadata};

use crate::generated_rs::{builtins_local, builtins_public};

const LARGEST_VERSION: u64 = 9007199254740991; // JavaScript Number.MAX_SAFE_INTEGER

// Used until a valid AWV_REG_TYPE_PUBLIC (or _LOCAL) has been hardcoded.
pub const AWV_REG_TYPE_DUMMY: &str =
    "ee383f084cffaab845617b1c43ffaee8b5c17e8fbbb3ad3d379c96b5b844f24e";

/// Check if this build of awe is compatible with the current network
#[cfg(not(feature = "skip-network-compatibility-check"))]
pub async fn is_compatible_network(client: &Client) -> bool {
    let xor_string = hard_coded_awv_type_string();
    if xor_string.len() == 0 {
        println!("ERROR: is_compatible_network() - no hard coded AWV type set.");
        return false;
    }

    let metadata_address =
        str_to_xor_name(xor_string.as_str()).expect("Failed to decode awv type string");

    get_website_metadata_from_network(metadata_address, client)
        .await
        .is_ok()
}

#[cfg(feature = "skip-network-compatibility-check")]
pub async fn is_compatible_network(_client: &Client) -> bool {
    return true;
}

fn hard_coded_awv_type_string() -> String {
    if builtins_local::AWV_REG_TYPE_LOCAL.len() > 0 {
        builtins_local::AWV_REG_TYPE_LOCAL.to_string()
    } else {
        builtins_public::AWV_REG_TYPE_PUBLIC.to_string()
    }
}

// Returns the value that indicates a register is an AWE versions register
//
// After Autonomi network launch this becomes fixed in code by setting the value of
// the value of AWV_REG_TYPE_PUBLIC which needs updating after each network reset.
//
// For local testing, the value of AWV_REG_TYPE_LOCAL can be set as a first default.
//
// AWV_REG_TYPE_PUBLIC is initially set to "" which, if AWV_REG_TYPE_LOCAL is also "" will
// return a dummy value kept constant for reference (AWV_REG_TYPE_DUMMY).
//
// A script is used to pick up this first site address and regenerate the code for AWV_REG_TYPE_PUBLIC
// and commit the change, so that subsequent builds of awe will have this register address
// hard coded.
//
// So the first site published by AWE will be special in that its register address is also used
// to indicate the AWE versions register type. Currently this value is not checked by AWE, but
// TODO: could be used in future to differentiate AWE versions registers from all other registers.
pub fn awv_register_type_string() -> String {
    let awv_string = hard_coded_awv_type_string();
    if awv_string.len() > 0 {
        awv_string.to_string()
    } else {
        let awv_type = AWV_REG_TYPE_DUMMY.to_string();
        println!(
            "==================================================================================="
        );
        println!("AWV_REG_TYPE_DUMMY: {}", &awv_type);
        println!(
            "WARNING: this should not appear when using an awe build configured for this network"
        );
        println!(
            "==================================================================================="
        );
        awv_type
    }
}

/// Store for a given website version (used for caching)
struct SiteVersion {
    version: u64,
    metadata_address: XorName,
    metadata: Option<WebsiteMetadata>,
}
impl SiteVersion {
    pub fn new(
        version: u64,
        metadata_address: XorName,
        metadata: Option<WebsiteMetadata>,
    ) -> SiteVersion {
        SiteVersion {
            version,
            metadata_address: metadata_address,
            metadata,
        }
    }
}
pub struct WebsiteVersions {
    default_version: Option<u64>, // Used for operations when no version is specified

    /// Cached data for the selected version
    site_version: Option<SiteVersion>,

    versions_register: VersionsRegister,
}

/// API for handling a versioned website
/// The owner_secret is only required for publish/update.
impl WebsiteVersions {
    /// Create a versioning register for a website,
    /// accessible using the returned WebsiteVersions
    pub async fn new_register(
        client: &Client,
        wallet: &mut Wallet,
        register_type: &XorName,
        owner_secret: Option<RegisterSecretKey>,
    ) -> Result<WebsiteVersions> {
        let mut versions_register =
            match VersionsRegister::new(client.clone(), None, owner_secret, &wallet).await {
                Ok(versions_register) => versions_register,
                Err(e) => {
                    println!("Failed to create website versions register online. {:?}", e);
                    return Err(eyre!("{e:?}"));
                }
            };

        let owner_secret = versions_register.owner_secret()?;
        versions_register
            .add_xor_name(client, register_type, &owner_secret, &wallet)
            .await?;

        Ok(WebsiteVersions {
            default_version: None,
            site_version: None,
            versions_register,
        })
    }

    /// Load a versions register and return wrapped in WebsiteVersions
    /// The owner_secret is only required for publish/update.
    pub async fn load_register(
        register_address: RegisterAddress,
        client: &Client,
        owner_secret: Option<RegisterSecretKey>,
    ) -> Result<WebsiteVersions> {
        // Check it exists to avoid accidental creation (and payment)
        let result = client.register_get(register_address).await;
        let versions_register = if result.is_ok() {
            VersionsRegister::from_client_register(result.unwrap(), owner_secret)
        } else {
            println!("DEBUG: load_register() error:");
            return Err(eyre!("register not found on network"));
        };

        let default_version = versions_register.num_versions()?;
        Ok(WebsiteVersions {
            default_version: Some(default_version),
            site_version: None,
            versions_register,
        })
    }

    pub fn versions_address(&self) -> &RegisterAddress {
        self.versions_register.address()
    }

    /// Publishes a new version pointing to the metadata provided
    /// which becomes the newly selected version
    /// Returns the selected version as a number
    pub async fn publish_new_version(
        &mut self,
        client: &Client,
        website_metadata: &XorName,
        wallet: &Wallet,
    ) -> Result<u64> {
        let owner_secret = self.versions_register.owner_secret()?;
        self.versions_register
            .add_xor_name(client, website_metadata, &owner_secret, wallet)
            .await?;
        println!("website_metadata added to register: {website_metadata:64x}");
        let version = self.versions_register.num_versions()?;
        self.default_version = Some(version);

        self.site_version = Some(SiteVersion::new(version, website_metadata.clone(), None));
        self.default_version = Some(version);
        Ok(version)
    }

    async fn lookup_resource(
        &mut self,
        resource_path: &String,
        version: Option<u64>,
        client: &Client,
    ) -> Result<XorName, StatusCode> {
        if !self.fetch_version_metadata(client, version).await.is_err() {
            if self.site_version.is_some() && self.site_version.as_ref().unwrap().metadata.is_some()
            {
                let site_version = self.site_version.as_ref().unwrap();
                let metadata = site_version.metadata.as_ref().unwrap();
                return metadata.lookup_resource(resource_path);
            } else {
                println!("Failed to fetch metadata.");
            }
        }
        Err(StatusCode::NOT_FOUND)
    }

    /// Makes the given version current by retrieving and storing website metadata.
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
    ) -> Result<()> {
        println!(
            "DEBUG fetch_version_metadata() self.site_version.is_some(): {}",
            self.site_version.is_some()
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
        if let Some(site) = &self.site_version {
            if site.version == version && site.metadata.is_some() {
                set_version_loaded(version);
                return Ok(());
            }
        }

        let metadata_xor = match self.get_metadata_address_from_register(version).await {
            Ok(metadata_xor) => metadata_xor,
            Err(e) => {
                println!("select_version() failed to get version {version} from register");
                return Err(eyre!(e));
            }
        };

        let metadata = match get_website_metadata_from_network(metadata_xor, client).await {
            Ok(metadata) => {
                set_version_loaded(version);
                metadata
            }
            Err(e) => {
                println!("select_version() failed to get website metadata from network");
                return Err(eyre!(e));
            }
        };

        self.site_version = Some(SiteVersion::new(version, metadata_xor, Some(metadata)));
        Ok(())
    }

    // Operations which will be applied to the currently selected version
    pub async fn get_metadata_address_from_register(&self, version: u64) -> Result<XorName> {
        println!("DEBUG XXXXXX get_metadata_address_from_register(version: {version})");
        // Use cached site value if available
        if let Some(site) = &self.site_version {
            if site.version == version {
                println!("DEBUG XXXXXX get_metadata_address_from_register() returning cached metadata address: {}", &site.metadata_address);
                return Ok(site.metadata_address.clone());
            }
        };
        self.versions_register.get_version_entry(version)
    }
}

struct VersionsRegister {
    register: Register,
    /// owner_secret is only required for publish/update (not access)
    owner_secret: Option<RegisterSecretKey>,
}

impl VersionsRegister {
    /// Gets an existing Register or creates a new register online
    /// The owner_secret is required when creating and for adding entries (publish/update)
    pub async fn new(
        client: Client,
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
            Ok(VersionsRegister {
                register: register.unwrap(),
                owner_secret: register_signing_key,
            })
        } else {
            Err(register.unwrap_err().into())
        }
    }

    /// The owner_secret is only required for publish/update (not access)
    pub fn from_client_register(
        client_register: Register,
        owner_secret: Option<RegisterSecretKey>,
    ) -> VersionsRegister {
        VersionsRegister {
            register: client_register,
            owner_secret: owner_secret.clone(),
        }
    }

    pub fn address(&self) -> &RegisterAddress {
        self.register.address()
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

        set_version_max(max_version as u64);

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

    /// Adds an XorName to the register, merging any branches
    pub async fn add_xor_name(
        &mut self,
        client: &Client,
        xor_value: &XorName,
        owner_secret: &RegisterSecretKey,
        _wallet: &Wallet, // Include for when updates are charged for
    ) -> Result<()> {
        let register_xor_address = self.register.address().to_hex();
        println!("Updating register    : {register_xor_address}");
        // The first register_get() has been added for testing (as reg_update() isn't always changing some registers)
        match client.register_get(self.register.address().clone()).await {
            Ok(register) => {
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

    fn owner_secret(&self) -> Result<RegisterSecretKey, Error> {
        match self.owner_secret.clone() {
            Some(owner_secret) => Ok(owner_secret),
            None => Err(eyre!(
                "ERROR: VersionsRegister can't update register without ::owner_secret"
            )),
        }
    }
}

/// Helper which gets a website version
/// Returns XorName of resource
pub async fn lookup_resource_for_website_version(
    resource_path: &String,
    versions_xor_name: RegisterAddress,
    version: Option<u64>,
    client: &Client,
) -> Result<XorName, StatusCode> {
    println!("DEBUG lookup_resource_for_website_version() version {version:?}");
    println!("DEBUG versions_xor_name: {versions_xor_name}");
    println!("DEBUG resource_path    : {resource_path}");

    match WebsiteVersions::load_register(versions_xor_name, client, None).await {
        Ok(mut website_versions) => {
            return website_versions
                .lookup_resource(resource_path, version, client)
                .await;
        }
        Err(e) => {
            println!("Failed to load versions register: {e:?}");
            return Err(StatusCode::NOT_FOUND);
        }
    };
}
