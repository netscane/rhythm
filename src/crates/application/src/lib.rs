pub mod auth;
pub mod command;
pub mod context;
pub mod error;
pub mod event;
pub mod projector;
pub mod query;
pub mod shared;

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
