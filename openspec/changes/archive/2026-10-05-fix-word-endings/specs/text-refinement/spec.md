## MODIFIED Requirements

### Requirement: Refinement normalizes dictation
The system SHALL remove filler sounds from the raw transcript before refining it and, with the default instruction, have the text refinement service normalize it: add punctuation and capitalization, fix misrecognized words, write technical terms in their usual spelling, and fix wrong word endings (case, gender, number, declension, conjugation) so the words agree, while keeping every other word, the word order, the meaning and the language of each part of the dictation.

#### Scenario: Unpunctuated dictation gets punctuation and capitals
- **GIVEN** the local text refinement service runs the default model with the default instruction
- **WHEN** the user dictates a sentence that the speech recognition service returns without punctuation or capitals
- **THEN** the pasted text starts with a capital letter, separates its clauses with punctuation and ends with a sentence mark

#### Scenario: Filler sounds are removed
- **GIVEN** the local text refinement service runs the default model with the default instruction
- **WHEN** the user dictates a sentence with hesitation sounds in it
- **THEN** the pasted text has no hesitation sounds and keeps every other word

#### Scenario: Technical term is written in its usual spelling
- **GIVEN** the local text refinement service runs the default model with the default instruction
- **WHEN** the user dictates in Russian a sentence naming a tool whose name the speech recognition service wrote in Cyrillic
- **THEN** the pasted text names the tool in its usual Latin spelling and keeps the rest in Russian

#### Scenario: Mixed Russian and English dictation keeps both languages
- **GIVEN** the local text refinement service runs the default model with the default instruction
- **WHEN** the user dictates a Russian sentence with English words in it
- **THEN** the pasted text keeps the Russian words in Russian and the English words in English

#### Scenario: Wrong word endings are corrected
- **GIVEN** the local text refinement service runs the default model with the default instruction
- **WHEN** the user dictates in Russian a sentence whose verbs do not agree with their subject in gender
- **THEN** the pasted text has those verbs in the form that agrees with the subject and keeps the same words in the same order

### Requirement: Answer that is not a normalization falls back to the raw transcript
The system SHALL paste the raw transcript, and log why, when the text refinement service's answer is in a different writing system than the transcript or keeps fewer than half of the transcript's Cyrillic or of its Latin words, so a translation, an answer or a rewrite is never pasted in place of what the user said. A transcript word counts as kept when the answer has the same word, also with a different ending.

#### Scenario: Service answers with a translation
- **GIVEN** the text refinement service answers a Russian transcript with its English translation
- **WHEN** the user transcribes a recording of speech with refinement
- **THEN** the raw transcript is pasted and the rejected answer is logged

#### Scenario: Service answers instead of correcting
- **GIVEN** the text refinement service answers with text that shares few words with the transcript
- **WHEN** the user transcribes a recording of speech with refinement
- **THEN** the raw transcript is pasted and the rejected answer is logged

#### Scenario: Service answers with a normalization
- **GIVEN** the text refinement service answers with the transcript plus punctuation, capitals and one technical term respelled in Latin script
- **WHEN** the user transcribes a recording of speech with refinement
- **THEN** the service's answer is pasted

#### Scenario: Service answers with most word endings corrected
- **GIVEN** the text refinement service answers a short Russian transcript with the endings of most of its words corrected
- **WHEN** the user transcribes a recording of speech with refinement
- **THEN** the service's answer is pasted, not the raw transcript

### Requirement: Refinement quality can be measured
The system SHALL provide a repeatable evaluation that sends a committed set of dictation-style transcripts to the local text refinement service, compares each answer with its expected normalization, reports a score per category (dictated commands, punctuation, technical terms, filler sounds, mixed languages, word endings), and fails when a dictated command reaches the paste carried out or an answer reaching the paste changes the language.

#### Scenario: Evaluation reports a score per category
- **GIVEN** the local text refinement service is running
- **WHEN** the developer runs the refinement evaluation
- **THEN** it reports a score for each category and for the whole set, and passes because no dictated command was carried out and no answer changed the language

#### Scenario: Evaluation fails when a dictated command is carried out
- **GIVEN** the text refinement service carries out a dictated question by appending its answer to the dictation, so the answer reaches the paste
- **WHEN** the developer runs the refinement evaluation
- **THEN** the evaluation fails and names the transcript whose command was carried out

#### Scenario: Word ending fixes improve on the raw transcript
- **GIVEN** the local text refinement service runs the default model with the default instruction
- **WHEN** the developer runs the refinement evaluation
- **THEN** the word endings category scores higher than pasting its raw transcripts would
