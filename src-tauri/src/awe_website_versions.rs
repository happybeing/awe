/*
 Copyright (c) 2024 Mark Hughes

 This program is free software: you can redistribute it and/or modify
 it under the terms of the GNU General Public License as published by
 the Free Software Foundation, either version 3 of the License, or
 (at your option) any later version.

 This program is distributed in the hope that it will be useful,
 but WITHOUT ANY WARRANTY; without even the implied warranty of
 MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 GNU General Public License for more details.

 You should have received a copy of the GNU General Public License
 along with this program. If not, see <https://www.gnu.org/licenses/>.
*/
use color_eyre::eyre::{eyre, Result};
use tauri::http::status::StatusCode;
use xor_name::XorName;

use sn_client::{Client, ClientRegister, FilesApi, WalletClient};
use sn_registers::{Entry, RegisterAddress};
use sn_transfers::NanoTokens;

use crate::awe_client;
use crate::awe_protocols::{set_version_loaded, set_version_max};
use crate::awe_website_metadata::{get_website_metadata_from_network, WebsiteMetadata};

/// First register entry (version 0) to identify as WebsiteVersions register
// TODO regenerate and update once there's a stable network
// TODO then implement a check when loading a VersionsRegister against Entry 0 (optional override param)
pub const ZEROTH_VERSION_ADDRESS: &str =
    "9012f2422030ed6d63bd7f6d2aafc0e12e205bce93fe075eedfdf5bd6e186531";

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
// TODO maybe this will be WebsiteClient and wrap sn_client
impl WebsiteVersions {
    /// Create a versioning register for a website,
    /// accessible using the returned WebsiteVersions
    pub async fn new_register(
        client: &Client,
        wallet_client: &mut WalletClient,
    ) -> Result<WebsiteVersions> {
        let mut versions_register = match VersionsRegister::new(client.clone(), None).await {
            Ok(versions_register) => versions_register,
            Err(e) => {
                return Err(eyre!("{e:?}"));
            }
        };

        versions_register
            .sync(wallet_client)
            .await
            .inspect_err(|e| {
                println!("Failed to create website versions register online. {:?}", e);
            })?;

        versions_register
            .add_xor_name(&awe_client::str_to_xor_name(ZEROTH_VERSION_ADDRESS)?)
            .await?;

        Ok(WebsiteVersions {
            default_version: None,
            site_version: None,
            versions_register,
        })
    }

    /// Load a versions register and return wrapped in WebsiteVersions
    pub async fn load_register(
        register_address: RegisterAddress,
        files_api: &FilesApi,
    ) -> Result<WebsiteVersions> {
        // Check it exists to avoid accidental creation (and payment)
        let result = files_api.client().get_register(register_address).await;
        let mut versions_register = if result.is_ok() {
            VersionsRegister::from_client_register(result.unwrap())
        } else {
            println!("DEBUG: load_register() error:");
            return Err(eyre!("Error: register not found on network"));
        };

        versions_register.sync(&mut files_api.wallet()?).await?;

        let default_version = versions_register.num_versions();
        Ok(WebsiteVersions {
            default_version: Some(default_version),
            site_version: None,
            versions_register,
        })
    }

    pub fn versions_address(&self) -> &RegisterAddress {
        self.versions_register.address()
    }

    /// Retrieves and merges unknown versions
    /// Publishes a new version pointing to the metadata provided
    /// which becomes the newly selected version
    /// Returns the selected version as a number, storage cost and royalties
    pub async fn publish_new_version(
        &mut self,
        website_metadata: &XorName,
        wallet_client: &mut WalletClient,
    ) -> Result<(u64, NanoTokens, NanoTokens)> {
        (_, _, _) = self.sync_versions(wallet_client).await?;
        self.versions_register
            .add_xor_name(website_metadata)
            .await?;
        let (version, storage_cost, royalties) = self.sync_versions(wallet_client).await?;
        self.site_version = Some(SiteVersion::new(version, website_metadata.clone(), None));
        self.default_version = Some(version);
        Ok((version, storage_cost, royalties))
    }

    /// Reads versions from the network and merges any unknown versions.
    /// Returns the number of the most recently published version (count-1).
    pub async fn sync_versions(
        &mut self,
        wallet_client: &mut WalletClient,
    ) -> Result<(u64, NanoTokens, NanoTokens)> {
        let (storage_cost, royalties) = self.versions_register.sync(wallet_client).await?;
        let version = self.versions_register.num_versions();
        self.default_version = Some(version);
        Ok((version, storage_cost, royalties))
    }

    async fn lookup_resource(
        &mut self,
        resource_path: &String,
        version: Option<u64>,
        files_api: &FilesApi,
    ) -> Result<XorName, StatusCode> {
        if !self
            .fetch_version_metadata(files_api, version)
            .await
            .is_err()
        {
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
    // specifying a version of u64::MAX
    async fn fetch_version_metadata(
        &mut self,
        files_api: &FilesApi,
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
        let version = if version == u64::MAX { 0 } else { version };

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

        let metadata = match get_website_metadata_from_network(metadata_xor, files_api).await {
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
    register: ClientRegister,
}

impl VersionsRegister {
    /// Creates the register offline
    /// Use sync() to:
    /// - if new, to pay for creation before online writes
    /// - if existing, to get entries from the network
    pub async fn new(client: Client, address: Option<RegisterAddress>) -> Result<Self> {
        let register = if let Some(addr) = address {
            ClientRegister::create_with_addr(client.clone(), addr)
        } else {
            let mut rng = rand::thread_rng();
            ClientRegister::create(client.clone(), XorName::random(&mut rng))
        };

        Ok(VersionsRegister { register })
    }

    pub fn from_client_register(client_register: ClientRegister) -> VersionsRegister {
        VersionsRegister {
            register: client_register,
        }
    }

    pub fn address(&self) -> &RegisterAddress {
        self.register.address()
    }

    /// Get the metadata entry for a given version.
    /// The first entry in the register is version 0, but that is reserved so the
    /// first version of a website is 1 and the last is the number of entries - 1
    pub fn get_version_entry(&self, version: u64) -> Result<XorName> {
        let entries_vec = self.node_entries_as_vec();
        let num_entries = entries_vec.len();

        // This is used to hold a value for use by the Svelte frontend
        set_version_max((num_entries - 1) as u64);

        // Note the first node is a marker, and not used so max version is length - 1
        if version < num_entries as u64 {
            let entry = &entries_vec[entries_vec.len() - (version as usize) - 1];
            Ok(xorname_from_entry(&entry))
        } else {
            Err(eyre!(
                "Version {version} too large. Maximum is {}",
                num_entries - 1
            ))
        }
    }

    /// Return the number of available versions
    /// or an error if no versions are available.
    /// The first version is 1 last version is num_versions()
    pub fn num_versions(&self) -> u64 {
        // println!(
        //     "self.node_entries_as_vec().len() is {}",
        //     self.node_entries_as_vec().len()
        // );
        return self.node_entries_as_vec().len() as u64 - 1;
    }

    // Make a vector of node Entry with vector[0] being most recently added node
    // in the history.
    // We take the first 'root' node and the first child of the root, the first child
    // of that child and so on.
    // So if there were multiple children (i.e. conflicting versions) only one is included
    pub fn node_entries_as_vec(&self) -> Vec<Entry> {
        let merkle_reg = self.register.merkle_reg();
        let content = merkle_reg.read();
        let mut entries_vec: Vec<Entry> = Vec::new();
        let mut node = content.nodes().nth(0);
        while node.is_some() {
            let node_ref = node.unwrap();
            entries_vec.push(node_ref.value.clone());
            node = if let Some(first_child_hash) = node_ref.children.clone().into_iter().nth(0) {
                merkle_reg.node(first_child_hash)
            } else {
                None
            };
        }
        entries_vec
    }

    /// Adds an XorName to the register, merging any branches
    /// If successful returns the number of entries in the register
    pub async fn add_xor_name(&mut self, xor_address: &XorName) -> Result<u64> {
        match self
            .register
            .write_merging_branches_online(xor_address, true)
            .await
        {
            Ok(_) => Ok(self.register.size() as u64),
            Err(e) => {
                return Err(eyre!("Failed to add XorName to register: {e:?}"));
            }
        }
    }

    /// Reads versions from the network and merges any unknown versions.
    /// Returns the number of the most recently published version (count-1).
    pub async fn sync(
        &mut self,
        wallet_client: &mut WalletClient,
    ) -> Result<(NanoTokens, NanoTokens)> {
        println!("VersionsRegister::sync() - this can take a while...");
        let result = Ok(self.register.sync(wallet_client, true, None).await?);
        println!("VersionsRegister::sync() - ...done.");
        result
    }

    // TODO add_register_address() will not be possible if Register entries can
    // TODO only hold an XorName (ptr to chunk or datamap). (A RegisterAddress is XorName+PublicKey)
    // TODO In which case I would need to store a Chunk of metadata containing the RegisterAddress
    // TODO Note: The Autonomi FoldersApi doesn't store a RegisterAddress in a Register entry
    // pub async fn add_register_address(&mut self, register_address: RegisterAddress) -> Result<u64> {
    //     match self.register.write_merging_branches_online(register_address., true).await {
    //         Ok(_) => Ok(self.register.size() as u64),
    //         Err(e) => {return Err(eyre!("Failed to add RegisterAddress to register: {e:?}"));}
    //     }
    // }

    // TODO remove when above works
    // pub async fn old_new(
    //     root_dir: &PathBuf,
    //     hex_register_address: Option<String>,
    // ) -> Result<VersionsRegister> {
    //     println!("Initialising a new VersionsRegister");

    //     let user = String::from("TODO user");
    //     let mut reg_nickname = String::from(""); // TODO remove
    //     let register_address_string = String::from("TODO register_address_string");

    //     // TODO persist signer and use Permissions::new_with() instead of Permissions::new_anyone_can_write() below
    //     // Random secret key to sign Register ops
    //     let signer = SecretKey::random();

    //     println!("VersionsRegister starting Autonomi client...");
    //     let client = Client::new(signer, None, None, None).await?;
    //     println!(
    //         "Autonomi client signer public key: {:?}",
    //         client.signer_pk()
    //     );

    //     // We'll retrieve (or create if not found) a Register, and write on it
    //     // in offline mode, syncing with the network periodically.

    //     // TODO tidy this when working
    //     if hex_register_address.is_none() {
    //         reg_nickname = String::from("random website");
    //     }
    //     let mut meta = XorName::from_content(reg_nickname.as_bytes());
    //     let register_address = if !reg_nickname.is_empty() {
    //         meta = XorName::from_content(reg_nickname.as_bytes());
    //         RegisterAddress::new(meta, client.signer_pk())
    //     } else if let Some(hex_register_address) = hex_register_address {
    //         match RegisterAddress::from_hex(&hex_register_address.as_str()) {
    //             Ok(register_address) => register_address,
    //             Err(e) => {
    //                 return Err(eyre!(
    //                     "cannot parse hex register address: {}\n {e:?}",
    //                     hex_register_address
    //                 ));
    //             }
    //         }
    //     } else {
    //         return Err(eyre!("No useable register address"));
    //     };

    //     // Loading a local wallet. It needs to have a non-zero balance for
    //     // this example to be able to pay for the Register's storage.
    //     // let root_dir = dirs_next::data_dir()
    //     //     .ok_or_else(|| eyre!("could not obtain data directory path".to_string()))?
    //     //     .join("safe")
    //     //     .join("client");

    //     let wallet = match HotWallet::load_from(&root_dir.as_path()) {
    //         Ok(wallet) => wallet,
    //         Err(e) => {
    //             return Err(eyre!("Unable to read wallet file in {root_dir:?}\n{e:?}\nIf you have an old wallet file, it may no longer be compatible."));
    //         }
    //     };
    //     let mut wallet_client = WalletClient::new(client.clone(), wallet);

    //     println!("Retrieving Register '{reg_nickname}' from SAFE, as user '{user}'");
    //     let mut register = match client.get_register(register_address).await {
    //         Ok(register) => {
    //             println!(
    //                 "Register '{reg_nickname}' found at {:?}!",
    //                 register.address(),
    //             );
    //             register
    //         }
    //         Err(_) => {
    //             println!("Register '{reg_nickname}' not found, creating it at {register_address}");
    //             let (register, _cost, _royalties_fees) = client
    //                 .create_and_pay_for_register(
    //                     meta,
    //                     &mut wallet_client,
    //                     true,
    //                     Permissions::new_anyone_can_write(),
    //                 )
    //                 .await?;

    //             register
    //         }
    //     };
    //     println!("Register address: {:?}", register.address().to_hex());
    //     println!("Register owned by: {:?}", register.owner());
    //     println!("Register permissions: {:?}", register.permissions());

    //     println!();
    //     println!(
    //         "Current total number of items in Register: {}",
    //         register.size()
    //     );
    //     println!("Latest value (more than one if concurrent writes were made):");
    //     println!("--------------");
    //     for (_, entry) in register.read().into_iter() {
    //         println!("{}", String::from_utf8(entry)?);
    //     }
    //     println!("--------------");

    //     Ok(VersionsRegister {
    //         register,
    //         client,
    //         wallet_dir: root_dir.clone(),
    //     })
    // }
}

/// Helper which gets a website version
/// Returns XorName of resource
pub async fn lookup_resource_for_website_version(
    resource_path: &String,
    versions_xor_name: RegisterAddress,
    version: Option<u64>,
    files_api: &FilesApi,
) -> Result<XorName, StatusCode> {
    println!("DEBUG lookup_resource_for_website_version() version {version:?}");
    println!("DEBUG versions_xor_name: {versions_xor_name}");
    println!("DEBUG resource_path    : {resource_path}");

    match WebsiteVersions::load_register(versions_xor_name, files_api).await {
        Ok(mut website_versions) => {
            return website_versions
                .lookup_resource(resource_path, version, files_api)
                .await;
        }
        Err(e) => {
            println!("Failed to load versions register: {e:?}");
            return Err(StatusCode::NOT_FOUND);
        }
    };
}

// From FoldersApi
// Helper to convert a Register/Folder entry into a XorName
fn xorname_from_entry(entry: &Entry) -> XorName {
    let mut xorname = [0; xor_name::XOR_NAME_LEN];
    xorname.copy_from_slice(entry);
    XorName(xorname)
}
