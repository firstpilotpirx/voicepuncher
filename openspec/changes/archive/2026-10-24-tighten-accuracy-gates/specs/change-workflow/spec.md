## ADDED Requirements

### Requirement: The accuracy baseline is recorded, accepted and shown as a trend
Every accuracy run SHALL record its sample size and mean word error rate in its stage journal line. `just accuracy-baseline` SHALL write the baseline file from this checkout's last journaled run of each sample size, with that run's commit, and keep the entry of a sample size this checkout never measured. The stage summary (`just stage-times`) SHALL show, per sample size, the mean word error rate of the last runs with their commits.

#### Scenario: An accuracy run is journaled with its rate
- **WHEN** an accuracy stage measures a mean word error rate of 0.047 over 100 recordings
- **THEN** the run's journal line records the sample size 100 and the rate 0.047

#### Scenario: The baseline is accepted from the last run
- **GIVEN** a stage journal whose last run of this checkout measured 0.04 over 10 recordings at commit `abc1234`
- **WHEN** the developer runs `just accuracy-baseline`
- **THEN** the baseline for 10 recordings is 0.04 from commit `abc1234`
- **AND** the baseline for 100 recordings is unchanged

#### Scenario: The summary shows the accuracy trend
- **GIVEN** a stage journal with three measured runs of 100 recordings
- **WHEN** the developer runs `just stage-times`
- **THEN** the summary lists the three rates of 100 recordings with their commits, oldest first
