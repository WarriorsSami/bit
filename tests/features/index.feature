Feature: Index Operations
  As a developer using bit (git clone)
  I want to manage the staging area (index) identically to git
  So that my repository state is compatible with standard git tools

  Background:
    Given I have redirected the temp directory
    And I have a clean temporary directory
    And I have initialized a git repository with bit

  Rule: Index content must be identical to git's index

    Scenario Outline: Successfully add files to index
      Given I have <file_count> files in a <structure> project
      When I add all files to the index using bit
      And I recreate the same scenario using git for index comparison
      Then both index contents should be identical

      Examples:
        | file_count | structure |
        | 1          | flat      |
        | 3          | flat      |
        | 5          | nested    |

    Scenario: Add files incrementally to index
      Given I have 3 files in a flat project
      When I add the first file to the index using bit
      And I add the second file to the index using bit
      And I add the third file to the index using bit
      And I recreate the same scenario using git for index comparison
      Then both index contents should be identical

    Scenario: Replace file with directory
      Given I have a file named "test.txt" in the project
      When I add the file to the index using bit
      And I replace the file with a directory containing files
      And I add the directory to the index using bit
      And I recreate the same scenario using git for index comparison
      Then both index contents should be identical

    Scenario: Replace directory with file
      Given I have a directory "testdir" with files in the project
      When I add the directory to the index using bit
      And I replace the directory with a single file
      And I add the file to the index using bit
      And I recreate the same scenario using git for index comparison
      Then both index contents should be identical

  Rule: Index operations should handle edge cases gracefully

    Scenario: Add non-existent file is ignored
      Given I have 1 files in a flat project
      When I add the existing file to the index using bit
      And I try to add a non-existent file to the index using bit
      Then the operation should succeed without errors
      And the non-existent file should not be in the index

    Scenario: Add unreadable file is ignored
      Given I have an unreadable file in the project
      When I try to add the unreadable file to the index using bit
      Then the operation should succeed without errors
      And the unreadable file should not be in the index

  Rule: Concurrent index operations should maintain consistency

    Scenario: Concurrent add operations maintain consistency
      Given I have files "alice.rb" and "bob.py" in the project
      When multiple concurrent add operations are performed
      And I recreate the same scenario using git for index comparison
      Then both index contents should be identical
