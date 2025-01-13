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

use std::{fmt::Display, str::FromStr};

// define the organizations type
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum OrganizationType {
    Default,
    Custom,
    UserDefault,
}

impl From<OrganizationType> for i16 {
    fn from(org: OrganizationType) -> i16 {
        match org {
            OrganizationType::Default => 0,
            OrganizationType::Custom => 1,
            OrganizationType::UserDefault => 2,
        }
    }
}

impl From<i16> for OrganizationType {
    fn from(org: i16) -> Self {
        match org {
            0 => OrganizationType::Default,
            1 => OrganizationType::Custom,
            2 => OrganizationType::UserDefault,
            _ => OrganizationType::Custom,
        }
    }
}

impl FromStr for OrganizationType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "default" => Ok(OrganizationType::Default),
            "custom" => Ok(OrganizationType::Custom),
            "user_default" => Ok(OrganizationType::UserDefault),
            _ => Ok(OrganizationType::Custom),
        }
    }
}

impl Display for OrganizationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrganizationType::Default => write!(f, "default"),
            OrganizationType::Custom => write!(f, "custom"),
            OrganizationType::UserDefault => write!(f, "user_default"),
        }
    }
}
