// Copyright 2024 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under The General Public License (GPL), version 3.
// Unless required by applicable law or agreed to in writing, the SAFE Network Software distributed
// under the GPL Licence is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied. Please review the Licences for the specific language governing
// permissions and limitations relating to use of the SAFE Network Software.

use crate::helpers::autonomi::*;
use access::keys::{get_secret_key_from_env, load_evm_wallet_from_env};
use autonomi::{EvmNetwork, Wallet};
use wallet::fs::{select_wallet, select_wallet_private_key};

pub(crate) mod encryption;
pub(crate) mod error;
pub(crate) mod fs;
pub(crate) mod input;

pub const DUMMY_NETWORK: EvmNetwork = EvmNetwork::ArbitrumSepolia;

/// Load wallet from ENV or disk
pub(crate) fn load_wallet() -> color_eyre::Result<Wallet> {
    // First try wallet from ENV
    if let Ok(wallet) = load_evm_wallet_from_env() {
        return Ok(wallet);
    }

    let wallet = select_wallet()?;

    Ok(wallet)
}

/// Load wallet private key from ENV or disk
pub(crate) fn load_wallet_private_key() -> color_eyre::Result<String> {
    // First try wallet private key from ENV
    if let Ok(private_key) = get_secret_key_from_env() {
        return Ok(private_key);
    }

    let private_key = select_wallet_private_key()?;

    Ok(private_key)
}
