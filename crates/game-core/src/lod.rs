/// LOD distance thresholds in world units.
/// Each entry is (fade_start, fade_end) — the entity fades out between these distances.
/// The next LOD level fades in over the same range for smooth crossfading.
pub const LOD_DISTANCES: [(f32, f32); 4] = [
    (0.0, 0.0),       // LOD0: visible from 0, fades out at LOD1 start
    (50.0, 55.0),      // LOD1: fades in 50-55, fades out at LOD2 start
    (120.0, 130.0),    // LOD2: fades in 120-130, fades out at LOD3 start
    (250.0, 260.0),    // LOD3: fades in 250-260, fades out at max
];

/// Maximum distance before the lowest LOD disappears entirely.
pub const LOD_MAX_DISTANCE: (f32, f32) = (400.0, 420.0);

/// Parse the LOD level from an entity name.
///
/// Matches suffixes like `_LOD0`, `_LOD1`, `_LOD2`, `_LOD3`.
/// Returns `None` if no LOD suffix is found.
pub fn parse_lod_level(name: &str) -> Option<u32> {
    let name = name.trim();
    if let Some(idx) = name.rfind("_LOD") {
        name[idx + 4..].parse::<u32>().ok()
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_lod_levels() {
        assert_eq!(parse_lod_level("SM_Env_Tree_Birch_01_LOD0"), Some(0));
        assert_eq!(parse_lod_level("SM_Env_Tree_Birch_01_LOD1"), Some(1));
        assert_eq!(parse_lod_level("SM_Env_Tree_Birch_01_LOD2"), Some(2));
        assert_eq!(parse_lod_level("SM_Env_Tree_Birch_01_LOD3"), Some(3));
        assert_eq!(parse_lod_level("SM_Env_Tree_Birch_01_Branches_LOD0"), Some(0));
        assert_eq!(parse_lod_level("SM_Env_Tree_Birch_01"), None);
        assert_eq!(parse_lod_level("Terrain"), None);
    }
}
