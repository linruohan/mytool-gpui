# Requirements Document: Phase 1 Performance Optimization

## Introduction

This document specifies the requirements for Phase 1 of the MyTool GPUI performance optimization plan. The goal is to improve application response speed and smoothness through targeted optimizations to the TodoStore indexing system, observer pattern, database connection management, and performance monitoring infrastructure.

MyTool is a Rust-based todo application built with the GPUI framework. The current implementation suffers from performance bottlenecks including O(n) index rebuilds on every change, excessive view re-renders, and inefficient database connection handling. Phase 1 addresses these core issues to establish a solid performance foundation.

## Glossary

- **TodoStore**: The global state management structure that serves as the single source of truth for all task data
- **Observer**: A view component that subscribes to TodoStore changes and re-renders when notified
- **Index**: In-memory data structures (HashMap, HashSet) that optimize query performance by grouping tasks
- **Incremental Update**: Updating only the changed portions of data structures rather than rebuilding everything
- **Version Tracking**: A monotonically increasing counter used to detect state changes
- **Dirty Flag**: A marker indicating that cached data needs to be recomputed
- **DatabaseConnection**: The SeaORM connection object used for SQLite database operations
- **Performance Monitor**: A system for tracking and logging operation timing and performance metrics
- **Board**: A view component displaying filtered task lists (Inbox, Today, Scheduled, etc.)

## Requirements

### Requirement 1: Incremental Index Updates

**User Story:** As a developer, I want TodoStore index updates to be incremental, so that task operations complete quickly regardless of the total number of tasks.

#### Acceptance Criteria

1. WHEN a single task is added, THE TodoStore SHALL update only the relevant indexes without rebuilding all indexes
2. WHEN a single task is updated, THE TodoStore SHALL remove the old task from indexes and add the updated task to indexes without full rebuild
3. WHEN a single task is deleted, THE TodoStore SHALL remove the task from all relevant indexes without full rebuild
4. WHEN a task's project_id changes, THE TodoStore SHALL update only the project_index entries for the old and new projects
5. WHEN a task's section_id changes, THE TodoStore SHALL update only the section_index entries for the old and new sections
6. WHEN a task's checked status changes, THE TodoStore SHALL update only the checked_set without affecting other indexes
7. WHEN a task's pinned status changes, THE TodoStore SHALL update only the pinned_set without affecting other indexes
8. THE TodoStore SHALL maintain index consistency after all incremental update operations

### Requirement 2: Version Tracking for Cache Invalidation

**User Story:** As a developer, I want TodoStore to track version numbers, so that views can efficiently detect when data has changed and avoid unnecessary recomputation.

#### Acceptance Criteria

1. THE TodoStore SHALL maintain a version counter that increments on every state modification
2. WHEN any task is added, updated, or deleted, THE TodoStore SHALL increment the version counter
3. WHEN any project is added, updated, or deleted, THE TodoStore SHALL increment the version counter
4. WHEN any section is added, updated, or deleted, THE TodoStore SHALL increment the version counter
5. WHEN any label is added, updated, or deleted, THE TodoStore SHALL increment the version counter
6. THE TodoStore SHALL provide a method to retrieve the current version number
7. THE version counter SHALL be a monotonically increasing unsigned integer

### Requirement 3: Observer Pattern Optimization with Version Caching

**User Story:** As a user, I want the application UI to remain responsive, so that I can work efficiently without experiencing lag or stuttering.

#### Acceptance Criteria

1. WHEN a Board view observes TodoStore changes, THE Board SHALL cache the last known version number
2. WHEN TodoStore notifies observers, THE Board SHALL compare its cached version with the current TodoStore version
3. IF the cached version equals the current version, THEN THE Board SHALL skip all recomputation and re-rendering
4. IF the cached version differs from the current version, THEN THE Board SHALL update its cached version and recompute its data
5. WHEN a Board recomputes data, THE Board SHALL update only the specific data structures that depend on changed state
6. THE Board SHALL maintain separate version caches for different data types (items, projects, sections, labels)
7. WHEN multiple TodoStore changes occur in rapid succession, THE Board SHALL batch updates to minimize re-renders

### Requirement 4: Fine-Grained Event System

**User Story:** As a developer, I want a fine-grained event system, so that views can respond only to relevant changes rather than all TodoStore modifications.

#### Acceptance Criteria

1. THE TodoStore SHALL define a TodoStoreEvent enum with variants for different change types
2. THE TodoStoreEvent enum SHALL include ItemAdded, ItemUpdated, ItemDeleted, ItemChecked, ItemPinned variants
3. THE TodoStoreEvent enum SHALL include ProjectAdded, ProjectUpdated, ProjectDeleted variants
4. THE TodoStoreEvent enum SHALL include SectionAdded, SectionUpdated, SectionDeleted variants
5. THE TodoStoreEvent enum SHALL include LabelAdded, LabelUpdated, LabelDeleted variants
6. THE TodoStoreEvent enum SHALL include a BulkUpdate variant for batch operations
7. WHEN TodoStore state changes, THE TodoStore SHALL emit the appropriate event variant
8. THE event variants SHALL include only the minimal data needed (typically just IDs)

### Requirement 5: Database Connection Management

**User Story:** As a developer, I want efficient database connection management, so that the application doesn't leak connections or waste resources.

#### Acceptance Criteria

1. THE DBState SHALL wrap DatabaseConnection in Arc to enable efficient sharing
2. WHEN database operations are spawned, THE system SHALL clone the Arc pointer rather than the connection
3. THE DatabaseConnection SHALL be initialized once at application startup
4. THE DatabaseConnection SHALL remain valid for the entire application lifetime
5. WHEN async operations complete, THE Arc reference count SHALL decrement automatically
6. THE system SHALL NOT create multiple DatabaseConnection instances
7. THE system SHALL log warnings if connection operations fail

### Requirement 6: Performance Monitoring Infrastructure

**User Story:** As a developer, I want comprehensive performance monitoring, so that I can identify bottlenecks and track optimization effectiveness.

#### Acceptance Criteria

1. THE system SHALL provide a PerformanceMonitor struct with operation tracking capabilities
2. THE PerformanceMonitor SHALL provide a track_operation method that measures execution time
3. WHEN an operation exceeds 100ms, THE PerformanceMonitor SHALL log a warning with operation name and duration
4. WHEN an operation completes under 100ms, THE PerformanceMonitor SHALL log debug information
5. THE system SHALL use structured logging with the tracing crate
6. THE system SHALL instrument critical operations including add_item, update_item, remove_item, and all index operations
7. THE system SHALL log operation timing in milliseconds
8. THE system SHALL include operation context (item IDs, counts) in log messages

### Requirement 7: Performance Targets

**User Story:** As a user, I want fast application response times, so that the application feels smooth and responsive during normal use.

#### Acceptance Criteria

1. WHEN a task is added, THE operation SHALL complete in less than 50ms
2. WHEN a task is updated, THE operation SHALL complete in less than 50ms
3. WHEN a task is deleted, THE operation SHALL complete in less than 50ms
4. WHEN switching between Board views, THE view SHALL render in less than 100ms
5. WHEN TodoStore contains 1000 tasks, THE inbox_items query SHALL complete in less than 10ms
6. WHEN TodoStore contains 1000 tasks, THE today_items query SHALL complete in less than 10ms
7. THE application memory usage SHALL remain stable over extended use (no memory leaks)
8. THE application SHALL handle 10,000 tasks without performance degradation

### Requirement 8: Backward Compatibility

**User Story:** As a user, I want my existing data to work seamlessly, so that I don't lose any tasks or experience data corruption during the optimization.

#### Acceptance Criteria

1. THE optimized TodoStore SHALL maintain the same public API as the current implementation
2. THE optimized TodoStore SHALL work with existing SQLite database schema without migration
3. WHEN loading existing data, THE system SHALL populate all indexes correctly
4. THE optimized system SHALL preserve all existing task properties and relationships
5. THE optimized system SHALL maintain compatibility with existing view components
6. THE system SHALL NOT require users to export and re-import their data
7. THE system SHALL handle edge cases in existing data (null values, empty strings, missing fields)

### Requirement 9: Testing and Validation

**User Story:** As a developer, I want comprehensive test coverage, so that I can confidently deploy optimizations without introducing bugs.

#### Acceptance Criteria

1. THE system SHALL include unit tests for all incremental index update operations
2. THE system SHALL include unit tests for version tracking behavior
3. THE system SHALL include integration tests for observer pattern optimization
4. THE system SHALL include performance benchmark tests measuring operation timing
5. THE system SHALL include tests verifying index consistency after operations
6. THE system SHALL include tests for edge cases (empty stores, single items, large datasets)
7. THE system SHALL include tests for concurrent operations
8. THE test suite SHALL achieve at least 80% code coverage for modified components
