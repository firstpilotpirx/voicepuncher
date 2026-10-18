# Spec Delta

## ADDED Requirements

### Requirement: English words spelled in Cyrillic are written in Latin script
The system SHALL, with the default model and instruction, have the text refinement service write an English word that speech recognition spelled in Cyrillic letters in its English Latin spelling, and restore an English word misheard into a Cyrillic non-word when the rest of the sentence makes clear which word was said, while every common Russian loanword and every word with a Russian ending or suffix stays in Cyrillic.

#### Scenario: English word spelled in Cyrillic comes out in Latin script
- **GIVEN** the local text refinement service runs the default model with the default instruction
- **WHEN** the user dictates in Russian a request for feedback that speech recognition writes as «фидбек»
- **THEN** the pasted text has «feedback» in Latin script and keeps the rest in Russian

#### Scenario: Two English words in one sentence both come out in Latin script
- **GIVEN** the local text refinement service runs the default model with the default instruction
- **WHEN** the user dictates in Russian a choice between the manual and the automatic mode that speech recognition writes as «мануал» and «автоматик»
- **THEN** the pasted text has «manual» and «automatic» in Latin script

#### Scenario: Misheard English word is restored from context
- **GIVEN** the local text refinement service runs the default model with the default instruction
- **WHEN** the user dictates in Russian a comparison of microphone modes in which speech recognition wrote the word "dynamic" as «даймэк»
- **THEN** the pasted text has «dynamic» in Latin script and no «даймэк»

#### Scenario: Russian loanwords stay in Cyrillic
- **GIVEN** the local text refinement service runs the default model with the default instruction
- **WHEN** the user dictates in Russian a sentence with the loanwords «менеджер», «компьютер» and «интернет»
- **THEN** the pasted text has those three words in Cyrillic

#### Scenario: Russified English verbs stay in Cyrillic
- **GIVEN** the local text refinement service runs the default model with the default instruction
- **WHEN** the user dictates in Russian a sentence with the verbs «деплоим» and «закомитим», English stems with Russian endings
- **THEN** the pasted text has those verbs in Cyrillic

### Requirement: IT terms split into Cyrillic syllables are restored
The system SHALL, with the default model and instruction, have the text refinement service replace a run of Cyrillic syllables or sound-alike Russian words that sounds like a known English IT term and fits the topic of the sentence with that term in its usual Latin spelling, and keep ordinary Russian words that make sense as they are.

#### Scenario: Shell framework and theme spelled as syllables are restored
- **GIVEN** the local text refinement service runs the default model with the default instruction
- **WHEN** the user dictates in Russian that they installed oh-my-zsh and the Powerlevel10k theme, and speech recognition writes «о май зш» and «пауэрлевел десять к»
- **THEN** the pasted text has «oh-my-zsh» and «Powerlevel10k» in Latin script, ignoring letter case, and keeps the rest in Russian

#### Scenario: Product names spelled as syllables are restored
- **GIVEN** the local text refinement service runs the default model with the default instruction
- **WHEN** the user dictates in Russian that GitHub Actions and VS Code need updating, and speech recognition writes «гит хаб экшнс» and «вс код»
- **THEN** the pasted text has «GitHub Actions» and «VS Code» in Latin script

## MODIFIED Requirements

### Requirement: Refinement quality can be measured
The system SHALL provide a repeatable evaluation that sends a committed set of dictation-style transcripts to the local text refinement service, compares each answer with its expected normalization, reports a score per category (dictated commands, punctuation, technical terms, filler sounds, mixed languages, word endings, English words in Cyrillic), and fails when a dictated command reaches the paste carried out, an answer reaching the paste changes the language, or a Russian loanword marked to stay in Cyrillic reaches the paste in Latin script.

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

#### Scenario: English word fixes improve on the raw transcript
- **GIVEN** the local text refinement service runs the default model with the default instruction
- **WHEN** the developer runs the refinement evaluation
- **THEN** the English words in Cyrillic category scores higher than pasting its raw transcripts would

#### Scenario: Evaluation fails when a loanword is respelled in Latin script
- **GIVEN** the text refinement service writes the loanword «компьютер» as «computer»
- **WHEN** the developer runs the refinement evaluation
- **THEN** the evaluation fails and names the transcript whose loanword left Cyrillic

### Requirement: Answer that is not a normalization falls back to the raw transcript
The system SHALL paste the raw transcript, and log why, when the text refinement service's answer is in a different writing system than the transcript or keeps fewer than half of the transcript's Cyrillic or of its Latin words, so a translation, an answer or a rewrite is never pasted in place of what the user said. A transcript word counts as kept when the answer has the same word, also with a different ending, or when the answer respells it, alone or with its neighbours, as Latin words that sound alike; respelled words do not count toward a change of writing system.

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

#### Scenario: Service respells most of a short dictation in Latin script
- **GIVEN** the text refinement service answers a short Russian dictation with its English words and IT terms respelled in Latin script, so most of its letters are Latin
- **WHEN** the user dictates that sentence with refinement
- **THEN** the service's answer is pasted, not the raw transcript

#### Scenario: Service translates the Russian part of a mixed dictation
- **GIVEN** the text refinement service answers a Russian sentence with English words in it by translating its Russian words into English
- **WHEN** the user dictates that sentence with refinement
- **THEN** the raw transcript is pasted
