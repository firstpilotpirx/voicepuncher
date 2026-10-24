## MODIFIED Requirements

### Requirement: Transcription meets the accuracy bar
The system SHALL transcribe a fixed sample of the benchmark corpus, 10 or 100 recordings with Russian and English speech in equal shares, with a mean word error rate no worse than the accepted baseline of that sample size plus the margin. The baseline SHALL be kept in the repository with the commit it came from. The margin SHALL be 0.01 while the stage journal holds fewer than 10 measured runs of that sample size, and otherwise 1.5 times the standard deviation of the last 10, never below 0.01. A run better than the baseline by more than the margin SHALL pass and say how to accept the new baseline. A sample size with no baseline SHALL fail and say how to record one.

#### Scenario: Benchmark corpus is transcribed within the accuracy bar
- **GIVEN** a fixed sample of 100 recordings of the benchmark corpus, half Russian and half English, with reference transcripts
- **WHEN** the user transcribes every recording of the sample
- **THEN** the mean word error rate is no worse than the accepted baseline plus the margin

#### Scenario: A run worse than the baseline fails
- **GIVEN** an accepted baseline mean word error rate of 0.05 and a margin of 0.01
- **WHEN** a run measures 0.07
- **THEN** the run fails, naming the measured rate, the baseline, the margin and the commit of the baseline

#### Scenario: A run within the margin passes
- **GIVEN** an accepted baseline mean word error rate of 0.05 and a margin of 0.01
- **WHEN** a run measures 0.055
- **THEN** the run passes

#### Scenario: A clearly better run passes with a hint
- **GIVEN** an accepted baseline mean word error rate of 0.05 and a margin of 0.01
- **WHEN** a run measures 0.03
- **THEN** the run passes and says that `just accuracy-baseline` accepts the better rate

#### Scenario: The margin follows the measured noise
- **GIVEN** a stage journal with 10 measured runs of the sample size
- **WHEN** the margin is computed
- **THEN** it is 1.5 times the standard deviation of those 10 rates, or 0.01 when that is smaller

#### Scenario: A sample size without a baseline fails
- **GIVEN** a baseline file with no entry for the sample size of the run
- **WHEN** a run measures its mean word error rate
- **THEN** the run fails and says that `just accuracy-baseline` records one
