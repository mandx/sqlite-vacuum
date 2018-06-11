use std::path::PathBuf;

use super::SQLiteFile;

#[test]
fn test_path_accessor() {
    let path = PathBuf::from("/file");
    let db = SQLiteFile::new(&path);
    assert_eq!(db.path(), &path);
}
