#![cfg_attr(not(feature = "std"), no_std)]

use super::*;

pub const APP_ID: AppId = *b"seed    ";
pub const DEEP: u8 = 4;
pub const RANGE: usize = 2;
/// Number of valid shortest paths.
pub const MAX_SHORTEST_PATH: u32 = 100;

pub const MAX_HASH_COUNT: u32 = 16u32.pow(RANGE as u32);

#[derive(Encode, Decode, Clone, Default, Ord, PartialOrd, PartialEq, Eq, RuntimeDebug)]
pub struct Candidate<AccountId> {
    pub score: u64,
    pub pathfinder: AccountId,
}

#[derive(Encode, Decode, Clone, Default, Ord, PartialOrd, PartialEq, Eq, RuntimeDebug)]pub struct FullOrder(pub Vec<u8>);
impl FullOrder {
    pub fn to_u64(&mut self) -> Option<u64> {
        let len = self.0.len();
        if len > 8 {
            return None;
        }
        let mut arr = [0u8; 8];
        self.0.extend_from_slice(&arr[len..]);
        arr.copy_from_slice(self.0.as_slice());
        Some(u64::from_le_bytes(arr))
    }

    pub fn from_u64(from: &u64, deep: usize) -> Self {
        let mut full_order = FullOrder::default();
        if deep > 8 {
            full_order.0 = u64::to_le_bytes(*from).to_vec();
        } else {
            full_order.0 = u64::to_le_bytes(*from)[..deep].to_vec();
        }
        full_order
    }

    pub fn connect(&mut self, order: &Vec<u8>) {
        self.0.extend_from_slice(&order[..RANGE]);
    }

    pub fn connect_to_u64(&mut self, order: &Vec<u8>) -> Option<u64> {
        self.connect(order);
        self.to_u64()
    }
}

#[derive(Encode, Decode, Clone, Default, RuntimeDebug)]
pub struct ResultHash {
    pub order: [u8; RANGE],
    pub score: u64,
    pub hash: [u8; 8],
}

// TODO binary_search_by_key & sort_by_key

impl Eq for ResultHash {}

impl Ord for ResultHash {
    fn cmp(&self, other: &Self) -> Ordering {
        self.order.cmp(&other.order)
    }
}

impl PartialOrd for ResultHash {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for ResultHash {
    fn eq(&self, other: &Self) -> bool {
        self.order == other.order
    }
}

#[derive(Encode, Decode, Ord, PartialOrd, Eq, Clone, Default, PartialEq, RuntimeDebug)]
pub struct Path<AccountId> {
    pub nodes: Vec<AccountId>,
    pub total: u32,
}

