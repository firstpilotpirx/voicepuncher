# text-refinement Specification

## Purpose
Normalizes a raw transcript before it is pasted (punctuation, capitals, filler sounds, misrecognized words and technical terms, never its meaning or language) by delegating the work to a text refinement service that runs on the same machine, so the app never runs a language model itself.

## Requirements

### Requirement: Refinement through a local text refinement service
The system SHALL obtain cleaned text from the configured text refinement service using the OpenAI Chat Completions API, and SHALL NOT load a language model into its own process.

#### Scenario: Transcript is refined by the service
- **WHEN** a raw transcript is ready while the text refinement service is running
- **THEN** the text the service returns is pasted as the cleaned text

#### Scenario: App starts without a local model file
- **WHEN** the user starts the app with no language model on disk and the service running
- **THEN** the app starts and is ready to record

### Requirement: Transcript is passed as data, not as instructions
The system SHALL send the configured system prompt as the instruction and the raw transcript as clearly delimited data, so spoken words that sound like commands are corrected rather than obeyed.

#### Scenario: Dictated text sounds like a request
- **WHEN** the user dictates a sentence that asks for a translation
- **THEN** the text refinement service receives the sentence as the text to correct, apart from the correction instructions

#### Scenario: Dictated translation request is pasted, not carried out
- **GIVEN** the local text refinement service runs the default model with the default instruction
- **WHEN** the user dictates in Russian a request to translate the text into English
- **THEN** the pasted text is that request in Russian, with the same words, not an English translation

#### Scenario: Dictated order to ignore the rules is pasted, not carried out
- **GIVEN** the local text refinement service runs the default model with the default instruction
- **WHEN** the user dictates an order to ignore all previous instructions and explain a term instead
- **THEN** the pasted text is the dictated order, not an explanation of the term

### Requirement: Text refinement service stays on this machine
The system SHALL accept only a text refinement service address that resolves to this machine, so transcripts never leave it.

#### Scenario: Remote service address is rejected
- **WHEN** the user starts the app with a text refinement service address on another host
- **THEN** the app exits before recording anything and explains that only a local service is allowed

### Requirement: Unavailable text refinement service is reported at startup
The system SHALL check, when refinement is turned on, that the text refinement service is reachable and offers the configured model before it accepts the first recording, and SHALL exit with a message that names the address or the available models when it does not.

#### Scenario: Service is not running at startup
- **WHEN** the user starts the app with refinement turned on while no text refinement service listens at the configured address
- **THEN** the app exits within 3 seconds with a message naming the address and how to start the service

#### Scenario: Configured model is not offered
- **WHEN** the user starts the app with refinement turned on and a model name the running service does not offer
- **THEN** the app exits with a message listing the models the service offers

### Requirement: Refinement failure falls back to the raw transcript
The system SHALL paste the raw transcript when the text refinement service fails, times out, or returns text that is cut off or less than half as long as the transcript.

#### Scenario: Service stops between recordings
- **WHEN** the user finishes a recording after the text refinement service has stopped
- **THEN** the raw transcript is pasted and the failure is logged

#### Scenario: Service hits its output limit
- **WHEN** the service stops generating because it reached its output limit
- **THEN** the raw transcript is pasted instead of the cut-off text

### Requirement: Model reasoning is never pasted
The system SHALL remove a model's reasoning block from the service's answer before pasting, so only the corrected text reaches the user.

#### Scenario: Reasoning model is configured
- **WHEN** the service answers with a reasoning block followed by the corrected text
- **THEN** only the corrected text is pasted

### Requirement: Outdated text refinement settings are reported
The system SHALL refuse a configuration that still uses the removed in-process model settings and SHALL name the setting that replaces each one.

#### Scenario: Config still points at a model file
- **WHEN** the user starts the app with a configuration that sets a language model file path
- **THEN** the app exits with a message saying the model is now chosen when starting the service and naming the service address setting

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

### Requirement: Transcript after a long pause is refined
The system SHALL send a transcript to the text refinement service on a connection the service has not closed, whatever time passed since the previous refinement, so that a long pause between recordings does not make the raw transcript be pasted.

#### Scenario: Service closed the idle connection during a long pause
- **GIVEN** a transcript was refined
- **AND** the text refinement service has since reset the connection that refinement used
- **WHEN** the next transcript is refined
- **THEN** the refined text from the service is returned, not the raw transcript

### Requirement: A dropped connection to the text refinement service is retried once
The system SHALL send the refinement request once more, to another copy of the text refinement service when one is up, when the copy it took refuses or drops the connection, or sends no answer within the request time limit, and SHALL log the failed attempt. When no copy accepts connections, the system SHALL first wait up to 30 seconds for one to come back. Only when the second attempt fails too, or no copy comes back, does refinement fall back to the raw transcript.

#### Scenario: The text refinement service drops one connection
- **GIVEN** the text refinement service drops the first connection and then answers normally
- **WHEN** the user finishes a recording
- **THEN** the refined text is pasted and the dropped attempt is logged

#### Scenario: The text refinement service keeps dropping the connection
- **GIVEN** the text refinement service drops every connection
- **WHEN** the user finishes a recording
- **THEN** the request is sent twice in total and the raw transcript is pasted

#### Scenario: A copy of the text refinement service is killed during a refinement
- **GIVEN** the app started the text refinement service as two copies
- **WHEN** the copy refining a transcript is killed before it answers
- **THEN** the other copy refines the transcript, and the text is not reported as pasted without refinement

#### Scenario: Every copy is down while the app restarts them
- **GIVEN** no copy of the text refinement service accepts connections, and one comes back 3 seconds later
- **WHEN** the user finishes a recording
- **THEN** the refined text is pasted once the copy answers, and the wait is logged

#### Scenario: A copy that sends no answer is replaced by another
- **GIVEN** the text refinement service runs as two copies, and one sends no answer
- **WHEN** a refinement request reaches that copy
- **THEN** after the request time limit the other copy refines the transcript

### Requirement: English words spelled in Cyrillic are written in Latin script
The system SHALL, with the default model and instruction, have the text refinement service write an English word that speech recognition spelled in Cyrillic letters in its English Latin spelling, and restore an English word misheard into a Cyrillic non-word when the rest of the sentence makes clear which word was said, while every common Russian loanword and every word with a Russian ending or suffix stays in Cyrillic. This holds for dictation whose sentence structure and English words differ from every example the default instruction shows the model and from every word the instruction names.

#### Scenario: English word spelled in Cyrillic comes out in Latin script
- **GIVEN** the local text refinement service runs the default model with the default instruction
- **WHEN** the user dictates in Russian a request to add a retry to a request, a sentence unlike every example and every word of the default instruction, and speech recognition writes «ретрай»
- **THEN** the pasted text has «retry» in Latin script and keeps the rest in Russian

#### Scenario: Two English words in one sentence both come out in Latin script
- **GIVEN** the local text refinement service runs the default model with the default instruction
- **WHEN** the user dictates in Russian that a flag has only two values, enabled and disabled, a sentence unlike every example of the default instruction, and speech recognition writes «энейблд» and «дизейблд»
- **THEN** the pasted text has «enabled» and «disabled» in Latin script

#### Scenario: Misheard English word is restored from context
- **GIVEN** the local text refinement service runs the default model with the default instruction
- **WHEN** the user dictates in Russian that a service needs the debug log level, a sentence unlike every example and every word of the default instruction, in which speech recognition wrote the word "debug" as «дибаг»
- **THEN** the pasted text has «debug» in Latin script and no «дибаг»

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

### Requirement: IT terms split into Cyrillic syllables are restored
The system SHALL, with the default model and instruction, have the text refinement service replace a run of Cyrillic syllables or sound-alike Russian words that sounds like a known English IT term and fits the topic of the sentence with that term in its usual Latin spelling, and keep ordinary Russian words that make sense as they are. This holds for dictation whose sentence structure and IT terms differ from every example the default instruction shows the model and from every term the instruction names.

#### Scenario: Shell framework and theme spelled as syllables are restored
- **GIVEN** the local text refinement service runs the default model with the default instruction
- **WHEN** the user dictates in Russian that Bash has the framework Bash-it with the Powerline theme, a sentence unlike every example and every term of the default instruction, and speech recognition writes «баш ит» and «пауэр лайн»
- **THEN** the pasted text has «Bash-it» and «Powerline» in Latin script, ignoring letter case, and keeps the rest in Russian

#### Scenario: Product names spelled as syllables are restored
- **GIVEN** the local text refinement service runs the default model with the default instruction
- **WHEN** the user dictates in Russian that someone rewrote a project from JavaScript to TypeScript, a sentence unlike every example of the default instruction, and speech recognition writes «джава скрипт» and «тайп скрипт»
- **THEN** the pasted text has «JavaScript» and «TypeScript» in Latin script

### Requirement: Refinement is off unless the user turns it on
The system SHALL paste the raw transcript without contacting the text refinement service, and SHALL start even if no service is running, unless the user turns refinement on on the command line or in the configuration; turning refinement off on the command line SHALL win over the configuration.

#### Scenario: Default settings paste the raw transcript
- **GIVEN** no text refinement service is listening at the configured address
- **WHEN** the user transcribes a recording of speech with default settings
- **THEN** the raw transcript is pasted

#### Scenario: Refinement turned on in the configuration
- **GIVEN** the configuration turns refinement on
- **AND** no text refinement service is listening at the configured address
- **WHEN** the user starts the app
- **THEN** the app exits with a message naming the text refinement service address and how to start it

#### Scenario: Turning refinement off on the command line wins over the configuration
- **GIVEN** the configuration turns refinement on
- **AND** no text refinement service is listening at the configured address
- **WHEN** the user transcribes a recording of speech with refinement turned off on the command line
- **THEN** the raw transcript is pasted

### Requirement: Running the app with one command starts the text refinement service only when refinement is on
The one command that runs the app for real use SHALL start neither the text refinement service nor need its model on disk unless the user turns refinement on with that command, and SHALL start and use the service when the user does.

#### Scenario: One command without refinement
- **GIVEN** the text refinement model is not on disk
- **WHEN** the user runs the app with one command and default settings
- **THEN** the app starts, no text refinement service is started, and a dictation is pasted as its raw transcript

#### Scenario: One command with refinement turned on
- **WHEN** the user runs the app with one command and turns refinement on
- **THEN** the text refinement service is started and a dictation is pasted as refined text
