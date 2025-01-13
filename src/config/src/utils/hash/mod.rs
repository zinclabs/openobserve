// Copyright 2024 OpenObserve Inc.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

use argon2::{password_hash::SaltString, Algorithm, Argon2, Params, PasswordHasher, Version};

pub mod cityhash;
pub mod fnv;
pub mod gxhash;
pub mod murmur3;

pub trait Sum64 {
    fn sum64(&mut self, key: &str) -> u64;
}

pub fn get_passcode_hash(pass: &str, salt: &str) -> String {
    let t_cost = 4;
    let m_cost = 2048;
    let p_cost = 2;
    let params = Params::new(m_cost, t_cost, p_cost, None).unwrap();
    let ctx = Argon2::new(Algorithm::Argon2d, Version::V0x10, params);
    let password = pass.as_bytes();
    let salt_string = SaltString::encode_b64(salt.as_bytes()).unwrap();
    ctx.hash_password(password, &salt_string)
        .unwrap()
        .to_string()
}
