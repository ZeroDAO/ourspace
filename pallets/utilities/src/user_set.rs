#[cfg(feature = "std")]
use sp_std::{ops::*, vec::Vec};
use orml_utilities::OrderedSet;

pub type UserSet<T> = OrderedSet<T>;

pub trait UserSetExt<T> {
    fn len(&self) -> usize;
    fn sub_set(&mut self, value: &Vec<T>);
}

impl<T: Ord> UserSetExt<T> for UserSet<T> {

    /// Set 的长度
    fn len(&self) -> usize {
		self.0.len()
	}

    /// 分片二分法： Set A 为 Set B 的子集，B = B - A
    fn sub_set(&mut self, value: &Vec<T>) {
        if value.is_empty() {
            return;
        };
        let v_len = value.len();
        let range = self.0.len().div(v_len).add(1);
        value.iter().fold(0, |acc, v| {
            let mut pross = acc;
            while pross < self.0.len() {
                let end = pross.saturating_add(range).min(self.len());
                if self.0[pross] <= *v && self.0[end] >= *v {
                    if let Ok(index) = self.0[pross..end].binary_search(&v) {
                        pross = index + pross - 1;
                        self.0.remove(pross + 1);
                    }
                    break;
                }
                pross = end;
            }
            pross
        });
    }
}

#[cfg(test)]
mod tests {
	use super::*;

    #[test]
    fn len() {
        let set_1: UserSet<i32> = UserSet::from(vec![1, 2, 3, 4]);
        let set_2: UserSet<i32> = UserSet::from(vec![]);
        let set_3: UserSet<i32> = UserSet::from(vec![1, 2, 2, 3, 4, 4, 4]);

        assert_eq!(set_1.len(), 4);
        assert_eq!(set_2.len(), 0);
        assert_eq!(set_3.len(), 4);
    }

    #[test]
    fn sub_set() {
        let mut set_left: UserSet<i32> = UserSet::from(vec![1, 2, 3, 4, 5, 6]);
        let set_right= vec![2, 3, 4];
        set_left.sub_set(&set_right);
        assert_eq!(set_left, UserSet::from(vec![1, 5, 6]));
    }
}