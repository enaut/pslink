pub mod home;
pub mod list_links;
pub mod list_users;

/// Unwrap a result and return it's content, or return from the function with another expression.
#[macro_export]
macro_rules! unwrap_or_return {
    ( $e:expr, $result:expr) => {
        match $e {
            Ok(x) => x,
            Err(_) => return $result,
        }
    };
}

/// Unwrap a result and return it's content, or return from the function with another expression.
#[macro_export]
macro_rules! unwrap_or_send {
    ( $e:expr, $result:expr, $orders:expr) => {
        match $e {
            Some(x) => x,
            None => {
                $orders.send_msg($result);
                return;
            }
        }
    };
}
