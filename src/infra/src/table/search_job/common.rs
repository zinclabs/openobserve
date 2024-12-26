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

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Value {
    String(String),
    I64(i64),
}

impl Value {
    pub fn string(s: &str) -> Self {
        Self::String(s.to_string())
    }

    pub fn i64(i: i64) -> Self {
        Self::I64(i)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum OperatorType {
    Equal,
    NotEqual,    // curently not used
    GreaterThan, // curently not used
    LessThan,
}
