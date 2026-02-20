# Implementation Plan: Phase 1 Performance Optimization

## Overview

This implementation plan breaks down the Phase 1 performance optimization into discrete, incremental tasks. Each task builds on previous work and includes testing to validate correctness. The plan follows a bottom-up approach: first establishing the foundation (version tracking, index methods), then optimizing the core (TodoStore), then updating consumers (Board views), and finally adding monitoring infrastructure.

## Tasks

- [ ] 1. Add version tracking infrastructure to TodoStore
  - Add `version: usize` field to TodoStore struct
  - Implement `version()` getter method
  - Implement `increment_version()` private method with wrapping arithmetic
  - Initialize version to 0 in `new()` and `default()`
  - Add logging for version wraparound detection
  - _Requirements: 2.1, 2.6, 2.7_

- [ ]* 1.1 Write property test for version tracking
  - **Property 4: Version Increment on Mutation**
  - **Validates: Requirements 2.1, 2.2, 2.3, 2.4, 2.5**
  - Test that all mutation operations increment version by exactly 1
  - Use proptest to generate random operations
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5_

- [ ]* 1.2 Write property test for version monotonicity
  - **Property 5: Version Monotonicity**
  - **Validates: Requirements 2.7**
  - Test that version numbers increase monotonically across operation sequences
  - Test wraparound behavior at usize::MAX
  - _Requirements: 2.7_

- [ ] 2. Implement incremental index update helper methods
  - [ ] 2.1 Implement `add_to_indexes(&mut self, item: &Arc<ItemModel>)` method
    - Add item to project_index if project_id is non-empty
    - Add item to section_index if section_id is non-empty
    - Add item.id to checked_set if item.checked is true
    - Add item.id to pinned_set if item.pinned is true
    - Add defensive logging for empty IDs
    - _Requirements: 1.1, 1.4, 1.5, 1.6, 1.7_
  
  - [ ] 2.2 Implement `remove_from_indexes(&mut self, item: &Arc<ItemModel>)` method
    - Remove item from project_index, clean up empty entries
    - Remove item from section_index, clean up empty entries
    - Remove item.id from checked_set
    - Remove item.id from pinned_set
    - _Requirements: 1.3, 1.4, 1.5, 1.6, 1.7_
  
  - [ ] 2.3 Implement `update_item_in_indexes(&mut self, old_item: &Arc<ItemModel>, new_item: &Arc<ItemModel>)` method
    - Call remove_from_indexes for old_item
    - Call add_to_indexes for new_item
    - _Requirements: 1.2, 1.4, 1.5, 1.6, 1.7_

- [ ]* 2.4 Write property test for incremental index updates
  - **Property 2: Incremental Index Updates**
  - **Validates: Requirements 1.1, 1.2, 1.3**
  - Test that add/update/delete operations don't call rebuild_indexes
  - Verify only affected indexes are modified
  - _Requirements: 1.1, 1.2, 1.3_

- [ ]* 2.5 Write property test for indexed field changes
  - **Property 3: Indexed Field Changes**
  - **Validates: Requirements 1.4, 1.5, 1.6, 1.7**
  - Test that changing project_id only affects project_index
  - Test that changing section_id only affects section_index
  - Test that changing checked only affects checked_set
  - Test that changing pinned only affects pinned_set
  - _Requirements: 1.4, 1.5, 1.6, 1.7_

- [ ] 3. Update TodoStore mutation methods to use incremental updates
  - [ ] 3.1 Update `add_item(&mut self, item: Arc<ItemModel>)` method
    - Call add_to_indexes instead of rebuild_indexes
    - Call increment_version at the end
    - Add performance monitoring instrumentation
    - _Requirements: 1.1, 2.2_
  
  - [ ] 3.2 Update `update_item(&mut self, item: Arc<ItemModel>)` method
    - Find old item and clone it
    - Update all_items vector
    - Call update_item_in_indexes with old and new items
    - Call increment_version at the end
    - Add performance monitoring instrumentation
    - _Requirements: 1.2, 2.2_
  
  - [ ] 3.3 Update `remove_item(&mut self, id: &str)` method
    - Find and clone item before removal
    - Remove from all_items vector
    - Call remove_from_indexes
    - Call increment_version at the end
    - Add performance monitoring instrumentation
    - _Requirements: 1.3, 2.2_
  
  - [ ] 3.4 Update project mutation methods (add_project, update_project, remove_project)
    - Add increment_version calls to each method
    - _Requirements: 2.3_
  
  - [ ] 3.5 Update section mutation methods (add_section, update_section, remove_section)
    - Add increment_version calls to each method
    - _Requirements: 2.4_
  
  - [ ] 3.6 Update label mutation methods (add_label, update_label, remove_label)
    - Add increment_version calls to each method
    - _Requirements: 2.5_

- [ ]* 3.7 Write property test for index consistency invariant
  - **Property 1: Index Consistency Invariant**
  - **Validates: Requirements 1.8**
  - Test that indexes always match rebuild_indexes result
  - Generate random operation sequences
  - Compare incremental updates vs full rebuild
  - _Requirements: 1.8_

- [ ] 4. Checkpoint - Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [ ] 5. Add TodoStoreEvent enum for fine-grained events
  - Define TodoStoreEvent enum with variants: ItemAdded, ItemUpdated, ItemDeleted, ItemChecked, ItemPinned, ProjectAdded, ProjectUpdated, ProjectDeleted, SectionAdded, SectionUpdated, SectionDeleted, LabelAdded, LabelUpdated, LabelDeleted, BulkUpdate
  - Each variant should contain only minimal data (IDs as String)
  - Add emit_event method to TodoStore (placeholder implementation with debug logging)
  - Call emit_event in all mutation methods
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5, 4.6, 4.7, 4.8_

- [ ]* 5.1 Write unit tests for event system
  - Test that correct event variants are emitted for each operation
  - Test that events contain only IDs, not full objects
  - _Requirements: 4.7, 4.8_

- [ ] 6. Optimize database connection management
  - [ ] 6.1 Update DBState struct to wrap DatabaseConnection in Arc
    - Change `conn: DatabaseConnection` to `conn: Arc<DatabaseConnection>`
    - Add `new(conn: DatabaseConnection) -> Self` constructor
    - Add `connection(&self) -> Arc<DatabaseConnection>` method
    - _Requirements: 5.1, 5.2_
  
  - [ ] 6.2 Update database initialization code
    - Modify initialization to create Arc<DatabaseConnection>
    - Ensure only one DatabaseConnection is created at startup
    - _Requirements: 5.3, 5.6_
  
  - [ ] 6.3 Update all database operation call sites
    - Replace `db.conn.clone()` with `db.connection()`
    - Update async operations to use Arc<DatabaseConnection>
    - Add error logging for connection failures
    - _Requirements: 5.2, 5.7_

- [ ]* 6.4 Write unit tests for database connection management
  - Test that DBState wraps Arc<DatabaseConnection>
  - Test that connection() returns Arc clone
  - Test that only one DatabaseConnection is created
  - _Requirements: 5.1, 5.3, 5.6_

- [ ] 7. Implement PerformanceMonitor infrastructure
  - [ ] 7.1 Create performance module and PerformanceMonitor struct
    - Create `crates/mytool/src/performance/mod.rs`
    - Create `crates/mytool/src/performance/monitor.rs`
    - Define PerformanceMonitor struct
    - _Requirements: 6.1_
  
  - [ ] 7.2 Implement track_operation method
    - Implement `track_operation<F, R>(name: &str, f: F) -> R`
    - Measure execution time using Instant
    - Log warning if duration > 100ms
    - Log debug if duration <= 100ms
    - Use structured logging with tracing crate
    - Include operation name and duration in milliseconds
    - _Requirements: 6.2, 6.3, 6.4, 6.5, 6.7_
  
  - [ ] 7.3 Implement track_async_operation method
    - Implement async version of track_operation
    - Same logging behavior as sync version
    - _Requirements: 6.2, 6.3, 6.4_
  
  - [ ] 7.4 Add defensive error handling to monitoring
    - Wrap logging in catch_unwind to prevent monitoring from failing operations
    - _Requirements: 6.2_

- [ ]* 7.5 Write unit tests for PerformanceMonitor
  - Test that operations under 100ms log debug
  - Test that operations over 100ms log warning
  - Test that monitoring doesn't affect operation results
  - Test that monitoring errors don't fail operations
  - _Requirements: 6.3, 6.4_

- [ ] 8. Instrument critical TodoStore operations with performance monitoring
  - Wrap inbox_items, today_items, scheduled_items, completed_items, pinned_items, items_by_project, items_by_section in PerformanceMonitor::track_operation
  - Add operation context (item counts) to log messages
  - _Requirements: 6.6, 6.8_

- [ ] 9. Checkpoint - Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [ ] 10. Add version caching to InboxBoard
  - [ ] 10.1 Add cached_version field to InboxBoard struct
    - Add `cached_version: usize` field
    - Initialize to 0 in new() method
    - _Requirements: 3.1_
  
  - [ ] 10.2 Implement version check in observer callback
    - Get current TodoStore version
    - Compare with cached_version
    - Return early if versions match (skip recomputation)
    - Update cached_version if versions differ
    - _Requirements: 3.2, 3.3, 3.4_
  
  - [ ] 10.3 Extract view update logic to separate method
    - Create `update_view(&mut self, store: &TodoStore, window: &mut Window, cx: &mut Context<Self>)` method
    - Move all recomputation logic into this method
    - Call from observer only when version differs
    - _Requirements: 3.4_

- [ ]* 10.4 Write unit tests for InboxBoard version caching
  - Test that observer skips recomputation when version unchanged
  - Test that observer recomputes when version changes
  - Test that cached_version is updated correctly
  - _Requirements: 3.1, 3.2, 3.3, 3.4_

- [ ] 11. Add version caching to TodayBoard
  - Add cached_version field
  - Implement version check in observer
  - Extract update_view method
  - _Requirements: 3.1, 3.2, 3.3, 3.4_

- [ ] 12. Add version caching to ScheduledBoard
  - Add cached_version field
  - Implement version check in observer
  - Extract update_view method
  - _Requirements: 3.1, 3.2, 3.3, 3.4_

- [ ] 13. Add version caching to ProjectBoard
  - Add cached_version field
  - Implement version check in observer
  - Extract update_view method
  - _Requirements: 3.1, 3.2, 3.3, 3.4_

- [ ]* 13.1 Write property test for observer batching
  - **Property 8: Observer Batching**
  - **Validates: Requirements 3.7**
  - Test that multiple rapid mutations result in single re-render
  - _Requirements: 3.7_

- [ ] 14. Add performance benchmark tests
  - [ ]* 14.1 Write benchmark for mutation operations
    - **Property 13: Mutation Operation Performance**
    - **Validates: Requirements 7.1, 7.2, 7.3**
    - Benchmark add_item with 1000 existing items
    - Benchmark update_item with 1000 existing items
    - Benchmark remove_item with 1000 existing items
    - Assert all operations complete in < 50ms
    - _Requirements: 7.1, 7.2, 7.3_
  
  - [ ]* 14.2 Write benchmark for query operations
    - **Property 14: Query Operation Performance**
    - **Validates: Requirements 7.5, 7.6**
    - Benchmark inbox_items with 1000 items
    - Benchmark today_items with 1000 items
    - Assert all queries complete in < 10ms
    - _Requirements: 7.5, 7.6_
  
  - [ ]* 14.3 Write benchmark for view rendering
    - **Property 15: View Rendering Performance**
    - **Validates: Requirements 7.4**
    - Benchmark Board view creation and update
    - Assert rendering completes in < 100ms
    - _Requirements: 7.4_
  
  - [ ]* 14.4 Write scalability benchmark
    - **Property 16: Scalability**
    - **Validates: Requirements 7.8**
    - Test all operations with 10,000 items
    - Verify performance targets still met
    - _Requirements: 7.8_

- [ ] 15. Add backward compatibility tests
  - [ ]* 15.1 Write API compatibility tests
    - **Property 17: API Compatibility**
    - **Validates: Requirements 8.1, 8.5**
    - Test all existing public methods still work
    - Test method signatures unchanged
    - _Requirements: 8.1, 8.5_
  
  - [ ]* 15.2 Write data integrity tests
    - **Property 18: Data Integrity**
    - **Validates: Requirements 8.3, 8.4**
    - Test loading existing database data
    - Test all properties preserved
    - _Requirements: 8.3, 8.4_
  
  - [ ]* 15.3 Write edge case handling tests
    - **Property 19: Edge Case Handling**
    - **Validates: Requirements 8.7**
    - Test null project_id handling
    - Test empty section_id handling
    - Test missing fields handling
    - Use proptest for random edge cases
    - _Requirements: 8.7_

- [ ] 16. Add integration tests for end-to-end workflows
  - Test complete workflow: add item → update item → delete item
  - Test complete workflow: create project → add items → delete project
  - Test complete workflow: switch views → verify correct data displayed
  - Verify indexes remain consistent throughout
  - Verify version tracking works correctly
  - Verify performance monitoring logs appear
  - _Requirements: 1.8, 2.1, 6.3, 6.4_

- [ ] 17. Final checkpoint - Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [ ] 18. Update documentation
  - Add inline documentation for new methods
  - Update module-level documentation for todo_store.rs
  - Document version tracking behavior
  - Document performance monitoring usage
  - Add examples of incremental update usage
  - _Requirements: 8.1_

## Notes

- Tasks marked with `*` are optional test-related sub-tasks and can be skipped for faster MVP
- Each task references specific requirements for traceability
- Checkpoints ensure incremental validation at key milestones
- Property tests validate universal correctness properties with minimum 100 iterations
- Unit tests validate specific examples and edge cases
- Performance benchmarks validate timing requirements
- The implementation follows a bottom-up approach: foundation → core → consumers → monitoring
- All changes maintain backward compatibility with existing code
- Version tracking is the foundation for observer optimization
- Incremental index updates are independent of observer optimization
- Database connection optimization is independent of other changes
- Performance monitoring can be added incrementally without affecting functionality
