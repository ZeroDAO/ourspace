use crate::*;

pub const APP_ID: AppId = *b"seed    ";

/// 可容纳 (16 * RANGE) ^ DEEP * n 的路径数据
pub const DEEP: u8 = 4;

/// 每个深度的步数，总量为 16 * RANGE 。
pub const RANGE: usize = 2;

/// Number of valid shortest paths.
pub const MAX_SHORTEST_PATH: u32 = 100;

/// 编译期计算的最大哈希上传数量。
pub const MAX_HASH_COUNT: u32 = 16u32.pow(RANGE as u32);

/// 种子候选人。
#[derive(Encode, Decode, Clone, Default, Ord, PartialOrd, PartialEq, Eq, RuntimeDebug)]
pub struct Candidate<AccountId,BlockNumber> {
    /// 中心度得分
    pub score: u64,

    /// 提交该候选人的 `pathfinder` 。
    pub pathfinder: AccountId,

    /// 是否有挑战 。
    pub has_challenge: bool,

    /// 在哪一个区块添加。
    pub add_at: BlockNumber,
}

/// 某个深度下的完整序号
#[derive(Encode, Decode, Clone, Default, Ord, PartialOrd, PartialEq, Eq, RuntimeDebug)]
pub struct FullOrder(pub Vec<u8>);

impl FullOrder {
    /// 转换为 `u64` 格式，便于在挑战模块中保存。
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

    /// 将 `u64` 的数据转换为 `FullOrder`。
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

    /// 连接当前 `FullOrder` 和传入的 `order` 。
    pub fn connect(&mut self, order: &[u8]) {
        self.0.extend_from_slice(&order[..RANGE]);
    }

    /// 连接当前 `FullOrder` 和传入的 `order` ，并转换为 `u64` 。
    pub fn connect_to_u64(&mut self, order: &[u8]) -> Option<u64> {
        self.connect(order);
        self.try_to_u64()
    }
}

/// 用于用户提交的简化版 `ResultHash`。
#[derive(Encode, Decode, Clone, Ord, PartialOrd, PartialEq, Eq, RuntimeDebug)]
pub struct PostResultHash(pub [u8; RANGE], pub u64);

impl PostResultHash {
    /// 转换为 `ResultHash` 。
    pub fn to_result_hash(&self) -> ResultHash {
        ResultHash {
            order: self.0,
            score: self.1,
        }
    }
}


/// 保存序号和得分。
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

/// 路径数据，包括路径的节点集合，和经过路径两端点的最短路径条数。
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
