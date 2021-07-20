
use frame_system::{self as system};

impl<T> ProxyBalance<T> for {
    fn check(last: T::BlockNumber) -> bool {
        last + T::ProxyPeriod < system::Module::<T>::block_number()
    }

    fn checked_proxy_fee(last: T::BlockNumber) -> bool {
        last + T::ProxyPeriod < system::Module::<T>::block_number()
    }
}