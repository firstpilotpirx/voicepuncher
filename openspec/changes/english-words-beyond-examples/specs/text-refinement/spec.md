## MODIFIED Requirements

### Requirement: English words spelled in Cyrillic are written in Latin script
The system SHALL, with the default model and instruction, have the text refinement service write an English word that speech recognition spelled in Cyrillic letters in its English Latin spelling, and restore an English word misheard into a Cyrillic non-word when the rest of the sentence makes clear which word was said, while every common Russian loanword and every word with a Russian ending or suffix stays in Cyrillic. This holds for dictation whose sentence structure and English words differ from every example the default instruction shows the model.

#### Scenario: English word spelled in Cyrillic comes out in Latin script
- **GIVEN** the local text refinement service runs the default model with the default instruction
- **WHEN** the user dictates in Russian a request for feedback that speech recognition writes as «фидбек»
- **THEN** the pasted text has «feedback» in Latin script and keeps the rest in Russian

#### Scenario: Two English words in one sentence both come out in Latin script
- **GIVEN** the local text refinement service runs the default model with the default instruction
- **WHEN** the user dictates in Russian that a flag has only two values, enabled and disabled, a sentence unlike every example of the default instruction, and speech recognition writes «энейблд» and «дизейблд»
- **THEN** the pasted text has «enabled» and «disabled» in Latin script

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

#### Scenario: English words the instruction never shows come out in Latin script
- **GIVEN** the local text refinement service runs the default model with the default instruction
- **WHEN** the user dictates in Russian to turn shuffle on and repeat off in a music player, English words that neither the instruction nor its examples show, and speech recognition writes «шаффл» and «репит»
- **THEN** the pasted text has «shuffle» and «repeat» in Latin script and keeps the rest in Russian
