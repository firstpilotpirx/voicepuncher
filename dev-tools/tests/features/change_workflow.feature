@change_workflow @component
Feature: change-workflow

  Rule: Branches land in groups through the landing queue

    Scenario: A checked branch behind main joins the queue
      Given the project's kit configuration
      Then a branch lands through the landing queue in groups

    Scenario: A group lands after one full check
      Given the project's kit configuration
      Then a landing group is checked once with "just full-check" before it merges

    Scenario: Seven queued branches land as one group
      Given the project's kit configuration
      When 8 queued branches form the landing groups
      Then the first group holds 7 branches
      And the next group holds 1 branch

    Scenario: A branch without build inputs needs no full check
      Given the project's kit configuration
      Then a change to "docs/ubiquitous-language.md" is not a build input
      And a change to "openspec/specs/change-workflow/spec.md" is not a build input
      And a change to "dev-tools/src/main.rs" is a build input
      And a change without build inputs is checked with "just test"

  Rule: Up to seven changes run at once

    Scenario: Seven ready changes start together
      Given the project's kit configuration
      When the orchestrator has 7 ready changes and 0 running
      Then 7 changes are launchable

    Scenario: An eighth change waits for a free slot
      Given the project's kit configuration
      When the orchestrator has 1 ready changes and 7 running
      Then 0 changes are launchable

  Rule: Code stays within size and complexity budgets

    Scenario: A new function over budget fails the checks
      Given the workspace's clippy configuration
      Then a function may have at most 40 code lines, 5 parameters, nesting 4 and cognitive complexity 15
      And the workspace denies the lints of those budgets

    Scenario: A struct, a type, a file or a module directory over budget fails the checks
      Given the size check of the workspace
      Then a file may have at most 300 code lines, a record type 12 fields, a type 35 public members and a module directory 2000 code lines

    Scenario: A listed unit back within budget must leave the baseline
      Given the size baseline
      Then every unit it lists with a measure is over its budget

  Rule: Clippy findings are ratcheted in the lint stage

    Scenario: The lint stage runs the clippy groups against their baseline
      Given the analyzers' configuration
      Then the clippy ratchet turns on the pedantic and nursery groups and the lints of unwrap, expect, panic and undocumented unsafe blocks
      And unwrap, expect and panic count only outside the test paths
      And the lint recipe runs the analyzer "clippy-groups"

    Scenario: A finding that is gone must leave the baseline
      Given the clippy baseline
      Then every finding it lists names a file of the repository and a count above zero

  Rule: Dependencies, spelling and recipes are analyzed in the lint stage

    Scenario: A shell mistake in a recipe fails the lint stage
      Given the justfile
      Then the lint recipe runs the analyzers of unused dependencies, licenses and advisories, and the library's compatibility
      And the lint recipe runs the analyzers of spelling and of the recipes' shell

    Scenario: A missing analyzer names its install command
      Given the justfile
      Then every pinned analyzer the lint stage needs is checked by version, naming its install command
      And the analyzer check installs nothing

  Rule: Code metrics are reported and ratcheted

    Scenario: The report lists the worst functions and the modules
      Given the justfile
      Then the test recipe runs the stage "metrics"

    Scenario: A new complex function fails the metrics
      Given the metrics baseline
      Then every function it lists is over cyclomatic complexity 10
      And it lists the duplicated lines with their count

  Rule: A fast static check fails before any test runs

    Scenario: A function over budget fails the static check without a test
      Given a scratch workspace with a function of 41 code lines that the size baseline does not list
      When the size analyzer runs there
      Then it fails naming the function and its length
      And no test binary holds the size check

    Scenario: A new complex function in a changed file fails the static check
      Given a scratch repository whose branch adds a function of cyclomatic complexity 12 that the metrics baseline does not list
      When the metrics of the changed files run there
      Then it fails naming the function and its complexity

    Scenario: A compile error fails the static check
      Given a scratch crate with a source file that does not compile
      When the static recipe runs there
      Then it fails naming the compile error
      And the size and changed-metrics analyzers do not run

    Scenario: The test recipe stops at the static check
      Given a scratch repository whose static stage fails
      When the stage runner runs the stages of the test recipe there
      Then the run fails in its static stage
      And its test stage does not run

    Scenario: A clean tree passes the static check in seconds
      Given the justfile
      Then the static recipe checks formatting, compiles with clippy, then runs the size and changed-metrics analyzers
      And the static recipe stops at the first check that fails
      And the static recipe builds and runs no test
      And the size and changed-metrics analyzers pass on the repository within 10 seconds

  Rule: The full check runs only the stages an edit can affect

    Scenario: An overlay change skips the seams, outages and start cues
      Given main passed the full check
      And a branch that changes only the overlay's drawing code
      When the full check computes its stage keys on the branch
      Then the chunk seams, outages and start cue stages are reused from main's pass

    Scenario: A change to the chunking code runs the seams stage
      Given main passed the full check
      And a branch that changes how a dictation is cut into chunks
      When the full check computes its stage keys on the branch
      Then the chunk seams stage runs

    Scenario: A file no stage names runs every stage
      Given main passed the full check
      And a branch that adds a source file no stage's inputs name
      When the full check computes its stage keys on the branch
      Then every model stage runs

  Rule: The chunk seams stage runs a sample unless the chunking code changed

    Scenario: A branch outside the chunking code runs the seam sample
      Given main passed the full check
      And a branch that changes the voice control code but not the chunking code
      When the full check chooses its chunk seams stage on the branch
      Then it prints that it runs the seam sample
      And the seam sample runs only the sample's cases

    Scenario: A branch that changes the chunking code runs the whole matrix
      Given main passed the full check
      And a branch that changes how a dictation is cut into chunks
      When the full check chooses its chunk seams stage on the branch
      Then it prints that it runs the whole seam matrix

  Rule: Every pull request has a hosted check

    Scenario: A clean pull request turns the check green
      Given the hosted check's workflow
      Then it runs on every pull request and every push to main on a macOS runner

    Scenario: A formatting error turns the check red
      Given the hosted check's workflow
      Then it runs "just fmt", "just lint" and "just test" as steps of their own
      And the lint and test steps run also after an earlier step failed

    Scenario: The hosted check needs no model, service or lock
      Given the hosted check's workflow
      Then it runs without the machine lock and with the test profile that leaves out the owner's desktop tests

  Rule: Recipes run without the machine lock when told so

    Scenario: Without the machine lock the recipes run directly
      Given a scratch repository with a stand-in task runner
      When the locked recipe "_fmt" runs without the machine lock
      Then the task runner ran "_fmt"
      And no lock wrapper ran

    Scenario: The tests of the lock itself are left out without the lock
      Given a scratch repository with the project's test needs
      And the listed tests are one that needs "needs_machine_lock" and a plain one
      When the test selection is gated without the machine lock
      Then the selection leaves out the tests of "needs_machine_lock"
      And the run says it skipped 1 test for "needs_machine_lock"

    Scenario: Model tests skip on a machine without models
      Given a scratch repository with the project's test needs
      And the listed tests are one that needs "needs_kws_models" and a plain one
      When the test selection is gated with the model paths exported empty
      Then the selection leaves out the tests of "needs_kws_models"
      And the run says it skipped 1 test for "needs_kws_models"

  Rule: A missing machine lock names its install command

    Scenario: A fresh clone without the lock tool gets the install command
      Given a scratch repository with a stand-in task runner
      And no machine lock on the machine
      When the stage runner runs the stages "fmt e2e-smoke-voice"
      Then the run fails before its first stage
      And the failure names the command that installs the lock tool and the variable that runs without the lock

  Rule: Every unsafe site states why it is sound

    Scenario: An unsafe block without a SAFETY comment fails the checks
      Given the workspace's clippy configuration
      Then the workspace denies "undocumented_unsafe_blocks"
      And the workspace denies "multiple_unsafe_ops_per_block"

    Scenario: A new unsafe impl Send outside the allowlist fails the checks
      Given the allowlist of thread-crossing types
      Then every unsafe impl of Send or Sync in the workspace's code is on it

  Rule: Documents name only things that exist

    Scenario: Every named thing exists
      Given the justfile
      Then every recipe "AGENTS.md" names is a recipe of the justfile
      And every recipe "README.md" names is a recipe of the justfile

    Scenario: A document names a recipe that does not exist
      Given the justfile
      And a document line "Run `just no-such-recipe` before you merge."
      Then the recipes it names that do not exist are "no-such-recipe"

  Rule: Integration tests build into one binary per member

    Scenario: A test file the binary leaves out fails the checks
      Then the integration test binary of "app" declares every file of its directory
      And the integration test binary of "command-spotting" declares every file of its directory
      And the integration test binary of "e2e-test" declares every file of its directory

    Scenario: A one-file edit relinks one binary per member
      Then the tests folder of "app" holds only its one binary, its harnesses and their steps
      And the tests folder of "command-spotting" holds only its one binary, its harnesses and their steps
      And the tests folder of "e2e-test" holds only its one binary, its harnesses and their steps

    Scenario: A moved test keeps its isolation and limits
      Given the test runner's configuration
      Then the tests of the shared command spotting service in the binary "it" run one at a time

  Rule: One named test runs alone

    Scenario: A named unit test runs alone
      Given a scratch repository with a stand-in task runner
      When the one-test recipe plans the names "should_reject_empty_chunk"
      Then the plan selects the tests "test(should_reject_empty_chunk)"
      And the plan runs the stage "test" as the recipe "test-one"

    Scenario: A named scenario runs alone
      Given a scratch repository with a stand-in task runner
      When the one-test recipe runs the names "scenario: A paste lands"
      Then the task runner ran "_stages test"
      And the task runner was given only the scenario "A paste lands"

    Scenario: A name that matches nothing fails the run
      Given a scratch repository with a stand-in task runner
      When the one-test recipe runs the names "should_never_exist"
      Then the run fails
      And the failure names "should_never_exist" as never ran

    Scenario: A one-test run is journaled but never a pass
      Given a scratch repository with a stand-in task runner
      When the stage runner runs the stages "test" as a one-test run without the machine lock
      Then the journal holds the run under the recipe "test-one"
      And no pass was recorded

  Rule: The Red step of TDD runs only its new test

    Scenario: Red runs one test, not the suite
      Given a scratch repository with a stand-in task runner
      When the one-test recipe runs the names "should_fail_first"
      Then the task runner ran "_stages test" as the recipe "test-one"
      And the task runner was given only the test filter "test(should_fail_first)"

    Scenario: The full suite runs once per task group
      Given the agent instructions
      Then they run "just test" once per task group, before its commit
      And they run "just test-one" at the Red step

  Rule: E2E stages run alone on the machine

    Scenario: Two runs' model stages take turns
      Given a scratch repository with a stand-in task runner
      When the stage runner runs the stages "fmt e2e-smoke-voice"
      Then the model stages ran under the machine lock and the quiet lock, both exclusive

    Scenario: Builds still run beside each other
      Given a scratch repository with a stand-in task runner
      When the stage runner runs the stages "fmt e2e-smoke-voice"
      Then the stage "_fmt" ran under the quiet lock in the shared mode

    Scenario: The journal shows the lock wait and hold
      Given a scratch repository with a stand-in task runner
      When the stage runner runs the stages "fmt e2e-smoke-voice"
      Then the stage table shows the e2e lock's wait and hold
      And the journal records the e2e lock's wait, hold and when the hold began

    Scenario: The stage summary finds no overlap between runs that took turns
      Given a journal of a run on "fix/a" started "2026-10-23T10:00:00+02:00" holding the e2e lock from "2026-10-23T10:01:00+02:00" for 600 seconds
      And a journal of a run on "fix/c" started "2026-10-23T10:20:00+02:00" holding the e2e lock from "2026-10-23T10:21:00+02:00" for 300 seconds
      When the user summarizes the stage journal
      Then the summary says "e2e lock overlaps: none (2 holds)"

    Scenario: The stage summary names overlapping runs
      Given a journal of a run on "fix/a" started "2026-10-23T10:00:00+02:00" holding the e2e lock from "2026-10-23T10:01:00+02:00" for 600 seconds
      And a journal of a run on "fix/b" started "2026-10-23T10:08:00+02:00" holding the e2e lock from "2026-10-23T10:09:00+02:00" for 300 seconds
      When the user summarizes the stage journal
      Then the summary says "fix/a 2026-10-23T10:00:00+02:00 ↔ fix/b 2026-10-23T10:08:00+02:00: 2m 00s"

  Rule: A test that flakes outside the change is frozen, not filed

    Scenario: An out-of-scope failure is frozen and the change lands
      Given a scratch clone on the branch "fix/other" whose main freezes nothing
      When the session freezes "scenario: A paste lands" from "full-check", stage "e2e-smoke-voice", beside "two other checks"
      Then the branch's frozen list names "scenario: A paste lands" with the day, recipe, stage and what ran beside it
      And main's frozen list names "scenario: A paste lands"
      And the next run leaves out the scenario "A paste lands"

    Scenario: A failure in a test the change touched is not frozen away
      Given the agent instructions
      Then they keep a failure in a test the branch touched the change's to fix
      And they freeze only a failure in a test the branch did not touch

  Rule: A frozen test runs nowhere until it is unfrozen

    Scenario: A frozen scenario is skipped and named
      When this harness runs "Seven ready changes start together" and "An eighth change waits for a free slot" with the second frozen
      Then the harness ran "Seven ready changes start together"
      And the harness named "An eighth change waits for a free slot" as skipped because it is frozen

    Scenario: A frozen plain test does not run
      Given a scratch clone on the branch "fix/other" whose main freezes a plain test
      When the next run's test selection is made
      Then the selection leaves out the plain test

    Scenario: A freeze on main reaches another worktree
      Given a scratch clone on the branch "fix/other" whose main freezes nothing
      And main freezes "scenario: A paste lands" after the branch began
      When the next run's frozen tests are read
      Then the next run leaves out the scenario "A paste lands"

    Scenario: The fixing branch unfreezes the test
      Given a scratch clone on the branch "fix/paste" whose main freezes "scenario: A paste lands"
      And the branch removed "scenario: A paste lands" from its frozen list
      When the next run's frozen tests are read
      Then the next run runs the scenario "A paste lands"

  Rule: The frozen tests are listed and run on demand

    Scenario: The user lists the frozen tests
      Given a scratch repository with a stand-in task runner
      And the frozen list holds "scenario: A paste lands" frozen on "2026-10-20" in the stage "e2e-smoke-voice"
      And the frozen list holds a plain test frozen on "2026-10-14" in the stage "test"
      When the user lists the frozen tests
      Then both are shown, each with its day, recipe, stage and what ran beside it

    Scenario: The user runs only the frozen tests
      Given a scratch repository with a stand-in task runner
      And the frozen list holds "scenario: A paste lands" frozen on "2026-10-20" in the stage "e2e-smoke-voice"
      And the frozen list holds a plain test frozen on "2026-10-14" in the stage "test"
      When the user plans a run of the frozen tests
      Then the plan runs the scenario "A paste lands" alone in the stage "e2e-smoke-voice"

    Scenario: A check names tests frozen too long
      Given a scratch repository with a stand-in task runner
      And the frozen list holds "scenario: A paste lands" frozen on "2026-10-20" in the stage "e2e-smoke-voice"
      And the frozen list holds a plain test frozen on "2026-10-14" in the stage "test"
      When a check reads the frozen tests on "2026-10-23"
      Then it says 2 tests are frozen
      And it names the plain test as frozen for more than 7 days
      And it does not name "scenario: A paste lands" as frozen for more than 7 days

  Rule: A test whose precondition is missing is skipped and counted, never green

    Scenario: The unit run leaves out the tests that need the services and says so
      Given a scratch repository with the project's test needs
      And the listed tests are one that needs "needs_stt" and one that needs "needs_kws"
      When the test selection is gated without services
      Then the selection leaves out the tests of "needs_stt"
      And the selection leaves out the tests of "needs_kws"
      And the run says it skipped 1 test for "needs_stt"
      And the run says it skipped 1 test for "needs_kws"

    Scenario: A model stage runs the tests that need the services it started
      Given a scratch repository with the project's test needs
      And the listed tests are one that needs "needs_stt" and a plain one
      When the test selection is gated with every service started
      Then the selection keeps the tests of "needs_stt"

    Scenario: A test run without its need fails and names it
      When a test that needs the command spotting models runs without them
      Then the test fails naming the models' variable and "just models"

    Scenario: Missing user recordings leave the recording tests out with a count
      Given a scratch repository with the project's test needs
      And the listed tests are two that need "needs_voice_samples"
      When the test selection is gated without recordings
      Then the run says it skipped 2 tests for "needs_voice_samples"

  Rule: The desktop tests run in the full check when the permissions allow

    Scenario: The full check runs the desktop tests with the permissions granted
      Given a scratch repository with the project's test needs
      And a permission probe that finds every permission granted
      When the desktop stage runs
      Then the test runner ran the desktop tests
      And the justfile's full check runs the stage "it-real"

    Scenario: A missing permission skips the desktop stage and names it
      Given a scratch repository with the project's test needs
      And the listed tests are two that need "needs_desktop"
      And a permission probe that finds "Accessibility" missing
      When the desktop stage runs
      Then the stage skipped itself
      And the run names "Accessibility" as missing
      And the run says it skipped 2 tests for "needs_desktop"

    Scenario: A skipped desktop stage is never a pass
      Given a scratch repository with a stand-in task runner
      When the stage runner runs the stages "it-real" with the desktop stage skipping itself
      Then the stage table shows "it-real" skipped
      And the journal records the stage "it-real" as "skipped"

    Scenario: Key presses into the focused app stay out of the full check
      Given a scratch repository with the project's test needs
      And a permission probe that finds every permission granted
      When the desktop stage runs
      Then the test runner left out the tests of "needs_real_keyboard"

  Rule: Every test asserts something that can fail

    Scenario: A test without a check fails the architecture tests
      Given the workspace's members
      Then the check of every test's assertions reads every member's code

  Rule: Tests leave the user's clipboard alone

    Scenario: The paste tests run while the user has copied something
      Given the paste tests
      Then they paste on a pasteboard of their own
      And they never open the general pasteboard

  Rule: A step both harnesses use is written once

    Scenario: A step text defined in both harnesses fails the architecture tests
      Then no step text is defined both in "app/tests/steps" and in "e2e-test/tests/steps"

  Rule: The stage runner is a tested program

    Scenario: The justfile is a thin layer
      Given the justfile
      Then it holds no Perl
      And it holds fewer than 800 lines

    Scenario: A run's journal line keeps its format
      Given a scratch repository with a stand-in task runner
      When the stage runner runs the stages "fmt e2e-smoke-voice"
      Then the journal line holds the fields of the former runner in their order

    Scenario: The summary reads the journal the former runner wrote
      Given a journal line the former runner wrote for a failed check
      When the user summarizes the stage journal
      Then the summary counts 1 run of "check"
      And the summary names the failed test of that check

  Rule: A helper's failure surfaces unless it is tolerated on purpose

    Scenario: An uncommented tolerated failure fails the tests
      Given a recipe text with "rm z || true" and no comment on it or above it
      Then the tolerated failures without a reason are "rm z || true"

    Scenario: The justfile says why each tolerated failure does not matter
      Given the justfile
      Then the tolerated failures without a reason are none

  Rule: The accuracy baseline is recorded, accepted and shown as a trend

    Scenario: An accuracy run is journaled with its rate
      Given a scratch repository with a stand-in task runner
      When the stage runner runs the stages "e2e-quick" with the accuracy stage measuring 0.047 over 100 recordings
      Then the journal records the sample size 100 and the rate 0.047

    Scenario: The baseline is accepted from the last run
      Given a scratch worktree with the accuracy baseline 0.031 for 10 and 0.047 for 100 recordings
      And a journal of this worktree measuring 0.04 over 10 recordings at commit "abc1234"
      When the developer accepts the accuracy baseline
      Then the baseline for 10 recordings is 0.04 from commit "abc1234"
      And the baseline for 100 recordings is 0.047 from commit "5ee5fbd"

    Scenario: The summary shows the accuracy trend
      Given a journal measuring "0.050 aaa1111, 0.048 bbb2222, 0.047 ccc3333" over 100 recordings
      When the user summarizes the stage journal
      Then the summary shows "100 recordings   0.050 (aaa1111)  0.048 (bbb2222)  0.047 (ccc3333)"
