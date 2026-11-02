#[cfg(target_os = "windows")]
pub(super) fn grown_capacity(current: usize, required: usize) -> Option<usize> {
    if required <= current {
        return Some(current);
    }
    if required > i32::MAX as usize {
        return None;
    }
    Some(
        required
            .checked_next_power_of_two()
            .unwrap_or(required)
            .min(i32::MAX as usize),
    )
}

#[cfg(all(test, target_os = "windows"))]
mod buffer_capacity_tests {
    use super::grown_capacity;
    #[test]
    fn storage_grows_geometrically_reuses_capacity_and_handles_large_meshes() {
        assert_eq!(grown_capacity(0, 1), Some(1));
        assert_eq!(grown_capacity(128, 65), Some(128));
        assert_eq!(grown_capacity(128, 129), Some(256));
        assert_eq!(
            grown_capacity(0, i32::MAX as usize),
            Some(i32::MAX as usize)
        );
        assert_eq!(grown_capacity(0, i32::MAX as usize + 1), None);
    }
}
