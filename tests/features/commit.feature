Feature: Commit Operations
  As a developer using bit (git clone)
  I want to create commits that match git's behavior exactly
  So that my repository state is compatible with standard git tools

  Background:
    Given I have redirected the temp directory
    And I have a clean temporary directory
    And I have initialized a git repository with bit
    And I have random author credentials configured
    And I have a random commit message

  Rule: Commit objects must be identical to those created by git

    Scenario Outline: Successfully commit files with different project structures
      Given I have files in a <structure> project structure
      When I perform a complete commit workflow with bit
      And I recreate the same scenario using git for commit comparison
      Then both implementations should produce identical tree OIDs

      Examples:
        | structure |
        | flat      |
        | nested    |

  Rule: Commit output format must match git's conventions

    Scenario Outline: Commit output format validation
      Given I have <file_count> files in a <structure> structure
      And I have author credentials and commit message configured
      When I commit the files using bit for a complete commit
      Then the commit should be successful with standard git formatting
      And the output should match the pattern "<pattern>"

      Examples:
        | file_count | structure | pattern                                      |
        | 1          | flat      | ^\[.*\(root-commit\) [0-9a-f]{7}\] .+$       |
        | 3          | flat      | ^\[.*\(root-commit\) [0-9a-f]{7}\] .+$       |
        | 5          | nested    | ^\[.*\(root-commit\) [0-9a-f]{7}\] .+$       |
