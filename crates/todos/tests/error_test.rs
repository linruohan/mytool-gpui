#[cfg(test)]
mod tests {
    use todos::error::TodoError;

    #[test]
    fn test_database_error() {
        let err = TodoError::DatabaseError("test error".to_string());
        assert_eq!(err.to_string(), "Database error: test error");
    }

    #[test]
    fn test_not_found_error() {
        let err = TodoError::NotFound("item".to_string());
        assert_eq!(err.to_string(), "item not found");
    }

    #[test]
    fn test_id_not_found_error() {
        let err = TodoError::IDNotFound;
        assert_eq!(err.to_string(), "id not found");
    }

    #[test]
    fn test_already_exists_error() {
        let err = TodoError::AlreadyExists("item".to_string());
        assert_eq!(err.to_string(), "item already exists");
    }

    #[test]
    fn test_error_from_db_error() {
        let db_err = sea_orm::DbErr::RecordNotFound("test".to_string());
        let err = TodoError::from(db_err);
        assert!(matches!(err, TodoError::DbError(_)));
    }
}
