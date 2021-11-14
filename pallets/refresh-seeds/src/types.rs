use crate::*;

pub const APP_ID: AppId = *b"seed    ";

/// Can hold (16 * RANGE) ^ DEEP * n of path data
pub const DEEP: u8 = 4;

/// Number of steps per depth, for a total of 16 * RANGE.
pub const RANGE: usize = 2;

/// Number of valid shortest paths.
pub const MAX_SHORTEST_PATH: u32 = 100;

/// The maximum number of hash uploads.
pub const MAX_HASH_COUNT: u32 = 16u32.pow(RANGE as u32);

/// Seeded candidates.
#[derive(Encode, Decode, Clone, Default, Ord, PartialOrd, PartialEq, Eq, RuntimeDebug)]
pub struct Candidate<AccountId,BlockNumber> {
    /// Centrality score
    pub score: u64,

    /// Submit the candidate's `pathfinder`.
    pub pathfinder: AccountId,

    /// Whether challenged.
    pub has_challenge: bool,

    /// In which block to add.
    pub add_at: BlockNumber,
}

/// The full serial number at a given depth
#[derive(Encode, Decode, Clone, Default, Ord, PartialOrd, PartialEq, Eq, RuntimeDebug)]
pub struct FullOrder(pub Vec<u8>);

impl FullOrder {
    /// Convert to `u64` format for easy saving in the challenge module.
    pub fn try_to_u64(&mut self) -> Option<u64> {
        let len = self.0.len();
        if len > 8 {
            return None;
        }
        let mut arr = [0u8; 8];
        self.0.extend_from_slice(&arr[len..]);
        arr.copy_from_slice(self.0.as_slice());
        Some(u64::from_le_bytes(arr))
    }

    /// Converts data from `u64` to `FullOrder`.
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

    /// Connects the current `FullOrder` with the incoming `order`.
    pub fn connect(&mut self, order: &[u8]) {
        self.0.extend_from_slice(&order[..RANGE]);
    }

    /// Connects the current `FullOrder` with the incoming `order` 
    /// and converts it to `u64`.
    pub fn connect_to_u64(&mut self, order: &[u8]) -> Option<u64> {
        self.connect(order);
        self.try_to_u64()
    }
}

/// A simplified version of `ResultHash` for user submission.
#[derive(Encode, Decode, Clone, Ord, PartialOrd, PartialEq, Eq, RuntimeDebug)]
pub struct PostResultHash(pub [u8; RANGE], pub u64);

impl PostResultHash {
    /// Convert to `ResultHash`.
    pub fn to_result_hash(&self) -> ResultHash {
        ResultHash {
            order: self.0,
            score: self.1,
        }
    }
}


/// Save the serial number and score.
#[derive(Encode, Decode, Clone, Default, RuntimeDebug)]
pub struct ResultHash {
    pub order: [u8; RANGE],
    pub score: u64,
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

/// Path data, including the set of nodes of the path, and the number of shortest 
/// paths that pass through the two endpoints of the path.
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
        let order_u64 = full_order.try_to_u64();
        assert_eq!(order_u64, Some(0));

        let mut full_order = FullOrder::from_u64(&0u64,1);
        let connect_vec = vec![12u8,16u8];
        let order_u64 = full_order.connect_to_u64(&connect_vec).unwrap();
        assert_eq!(FullOrder::from_u64(&order_u64,2).0, connect_vec);
	}
}
