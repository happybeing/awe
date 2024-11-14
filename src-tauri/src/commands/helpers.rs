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

use autonomi::client::registers::Register;
use sn_registers::Entry;
use xor_name::XorName;

// Make a vector of node Entry with vector[0] being the first node in the history.
// We take the first 'root' node and the first child of the root, the first child
// of that child and so on.
// So if there were multiple children (i.e. conflicting versions) only one is included
pub fn node_entries_as_vec(register: &Register) -> Vec<Entry> {
    let merkle_reg = register.inner_merkle_reg();
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
    entries_vec.reverse();
    entries_vec
}

// From FoldersApi
// Helper to convert a Register/Folder entry into a XorName
pub fn xorname_from_entry(entry: &Entry) -> XorName {
    let mut xorname = [0; xor_name::XOR_NAME_LEN];
    xorname.copy_from_slice(entry);
    XorName(xorname)
}
