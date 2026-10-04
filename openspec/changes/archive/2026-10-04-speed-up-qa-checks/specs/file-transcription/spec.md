## MODIFIED Requirements

### Requirement: Transcription meets the accuracy bar
The system SHALL transcribe a fixed sample of the benchmark corpus, 100 recordings with Russian and English speech in equal shares, with a mean word error rate below 30 % after refinement.

#### Scenario: Benchmark corpus is transcribed within the accuracy bar
- **GIVEN** a fixed sample of 100 recordings of the benchmark corpus, half Russian and half English, with reference transcripts
- **WHEN** the user transcribes every recording of the sample with refinement
- **THEN** the mean word error rate is below 30 %
