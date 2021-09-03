#![cfg_attr(not(feature = "std"), no_std)]

use super::*;

pub const APP_ID: AppId = *b"seed    ";
pub const DEEP: u8 = 4;
pub const RANGE: usize = 2;
/// Number of valid shortest paths.
pub const MAX_SHORTEST_PATH: u32 = 100;

pub const MAX_HASH_COUNT: u32 = 16u32.pow(RANGE as u32);

#[derive(Encode, Decode, Clone, Default, Ord, PartialOrd, PartialEq, Eq, RuntimeDebug)]
pub struct Candidate<AccountId,BlockNumber> {
    pub score: u64,
    pub pathfinder: AccountId,
    pub has_challenge: bool,
    pub add_at: BlockNumber,
}

#[derive(Encode, Decode, Clone, Default, Ord, PartialOrd, PartialEq, Eq, RuntimeDebug)]
pub struct FullOrder(pub Vec<u8>);
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
        if deep <= 1 {
            return full_order;
        }
        let len = (deep - 1) * (RANGE as usize);
        if len > 10 {
            full_order.0 = u64::to_le_bytes(*from).to_vec();
        } else {
            full_order.0 = u64::to_le_bytes(*from)[..len].to_vec();
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

#[derive(Encode, Decode, Clone, Ord, PartialOrd, PartialEq, Eq, Default, RuntimeDebug)]
pub struct PostResultHash(pub String, pub u64, pub String);

impl PostResultHash {
    // TODO Adopt more efficient encoding
    pub fn to_result_hash(&self) -> Option<ResultHash> {
        // TODO too wordy 
        let order_slice = self.0.as_bytes();
        let hash_slice = self.2.as_bytes();
        if order_slice.len() != RANGE || hash_slice.len() != 8 {
            return None;
        }
        let mut order: [u8;RANGE] = Default::default();
        let mut hash: [u8;8] = Default::default();
        order.clone_from_slice(order_slice);
        hash.clone_from_slice(hash_slice);
        Some(ResultHash {
            order,
            score: self.1,
            hash,
        })
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


#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn from_u64() {
		let mut full_order = FullOrder::from_u64(&0u64,1);
        assert_eq!(full_order.0, Vec::<u8>::new());
        let order_u64 = full_order.to_u64();
        assert_eq!(order_u64, Some(0));

        let mut full_order = FullOrder::from_u64(&0u64,1);
        let connect_vec = vec![12u8,16u8];
        let order_u64 = full_order.connect_to_u64(&connect_vec).unwrap();
        assert_eq!(FullOrder::from_u64(&order_u64,2).0, connect_vec);
	}
}
