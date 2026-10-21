## MODIFIED Requirements

### Requirement: A missing program or model is named
A service whose program, or a model file named by its variable, cannot be found SHALL be Failed at once, without a start, with a reason naming what was looked for and where. A model of the catalog missing from the models folder does not reach the services: the runtime waits for models first.

#### Scenario: The refinement model is not on disk
- **GIVEN** the variable for the text refinement model names a file that is not on disk
- **WHEN** the services the configuration needs are looked up
- **THEN** the text refinement service is Failed with a reason naming the model file it looked for
