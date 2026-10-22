@text_refinement @e2e
Feature: text-refinement

  Rule: Refinement through a local text refinement service

    @smoke
    Scenario: Transcript is refined by the service
      Given the text refinement service answers with corrected text
      When the user transcribes a recording of speech with refinement
      Then the text the service returned is pasted

    @smoke
    Scenario: App starts without a local model file
      Given the text refinement service is running
      And no language model is on disk for the app
      When the user starts the app with refinement
      Then the app starts and is ready to record

  Rule: Transcript is passed as data, not as instructions

    @smoke
    Scenario: Dictated text sounds like a request
      Given the text refinement service answers with corrected text
      When the user dictates a sentence that asks for a translation
      Then the text refinement service receives the sentence as the text to correct, apart from the correction instructions

    @refinement
    Scenario: Dictated translation request is pasted, not carried out
      Given the local text refinement service runs the default model with the default instruction
      When the user dictates in Russian a request to translate the text into English
      Then the pasted text is that request in Russian, with the same words, not an English translation

    @refinement
    Scenario: Dictated order to ignore the rules is pasted, not carried out
      Given the local text refinement service runs the default model with the default instruction
      When the user dictates an order to ignore all previous instructions and explain a term instead
      Then the pasted text is the dictated order, not an explanation of the term

  Rule: Text refinement service stays on this machine

    @smoke
    Scenario: Remote service address is rejected
      Given the text refinement service address is on another host
      When the user starts the app with refinement
      Then the app exits before recording anything and explains that only a local text refinement service is allowed

  Rule: Unavailable text refinement service is reported at startup

    @smoke
    Scenario: Service is not running at startup
      Given no text refinement service is listening at the configured address
      When the user starts the app with refinement
      Then the app exits within 3 seconds with a message naming the text refinement service address and how to start it

    @smoke
    Scenario: Configured model is not offered
      Given the text refinement service offers only models other than the configured one
      When the user starts the app with refinement
      Then the app exits with a message listing the models the service offers

  Rule: Refinement failure falls back to the raw transcript

    @smoke
    Scenario: Service stops between recordings
      Given the text refinement service stops after the app has started
      When the user transcribes a recording of speech with refinement
      Then the raw transcript is pasted and the failure is logged

    @smoke
    Scenario: Service hits its output limit
      Given the text refinement service stops generating at its output limit
      When the user transcribes a recording of speech with refinement
      Then the raw transcript is pasted instead of the cut-off text

  Rule: Answer that is not a normalization falls back to the raw transcript

    @smoke
    Scenario: Service answers with a translation
      Given the text refinement service answers a Russian transcript with its English translation
      When the user transcribes a recording of speech with refinement
      Then the raw transcript is pasted and the rejected answer is logged

    @smoke
    Scenario: Service answers instead of correcting
      Given the text refinement service answers with text that shares few words with the transcript
      When the user transcribes a recording of speech with refinement
      Then the raw transcript is pasted and the rejected answer is logged

    @smoke
    Scenario: Service answers with a normalization
      Given the text refinement service answers with the transcript plus punctuation, capitals and one technical term respelled in Latin script
      When the user transcribes a recording of speech with refinement
      Then the service's answer is pasted

    @smoke
    Scenario: Service answers with most word endings corrected
      Given the text refinement service answers a short Russian transcript with the endings of most of its words corrected
      When the user transcribes a recording of speech with refinement
      Then the service's answer is pasted, not the raw transcript

    @smoke
    Scenario: Service respells most of a short dictation in Latin script
      Given the text refinement service answers a short Russian dictation with its English words and IT terms respelled in Latin script, so most of its letters are Latin
      When the user dictates that sentence with refinement
      Then the service's answer is pasted, not the raw transcript of that sentence

    @smoke
    Scenario: Service translates the Russian part of a mixed dictation
      Given the text refinement service answers a Russian sentence with English words in it by translating its Russian words into English
      When the user dictates that sentence with refinement
      Then the raw transcript of that sentence is pasted

  Rule: Refinement normalizes dictation

    @refinement
    Scenario: Unpunctuated dictation gets punctuation and capitals
      Given the local text refinement service runs the default model with the default instruction
      When the user dictates a sentence that the speech recognition service returns without punctuation or capitals
      Then the pasted text starts with a capital letter, separates its clauses with punctuation and ends with a sentence mark

    @refinement
    Scenario: Filler sounds are removed
      Given the local text refinement service runs the default model with the default instruction
      When the user dictates a sentence with hesitation sounds in it
      Then the pasted text has no hesitation sounds and keeps every other word

    @refinement
    Scenario: Technical term is written in its usual spelling
      Given the local text refinement service runs the default model with the default instruction
      When the user dictates in Russian a sentence naming a tool whose name the speech recognition service wrote in Cyrillic
      Then the pasted text names the tool in its usual Latin spelling and keeps the rest in Russian

    @refinement
    Scenario: Mixed Russian and English dictation keeps both languages
      Given the local text refinement service runs the default model with the default instruction
      When the user dictates a Russian sentence with English words in it
      Then the pasted text keeps the Russian words in Russian and the English words in English

    @refinement
    Scenario: Wrong word endings are corrected
      Given the local text refinement service runs the default model with the default instruction
      When the user dictates in Russian a sentence whose verbs do not agree with their subject in gender
      Then the pasted text has those verbs in the form that agrees with the subject and keeps the same words in the same order

  Rule: English words spelled in Cyrillic are written in Latin script

    @refinement
    Scenario: English word spelled in Cyrillic comes out in Latin script
      Given the local text refinement service runs the default model with the default instruction
      When the user dictates in Russian a request to add a retry that speech recognition writes in Cyrillic
      Then the pasted text has the English word retry in Latin script and keeps the rest in Russian

    @refinement
    Scenario: Two English words in one sentence both come out in Latin script
      Given the local text refinement service runs the default model with the default instruction
      When the user dictates in Russian that a flag has only two values, enabled and disabled, and speech recognition writes both in Cyrillic
      Then the pasted text has the English words enabled and disabled in Latin script

    @refinement
    Scenario: Misheard English word is restored from context
      Given the local text refinement service runs the default model with the default instruction
      When the user dictates in Russian that a service needs the debug log level and speech recognition misheard the word debug
      Then the pasted text has the English word debug in Latin script instead of the misheard word

    @refinement
    Scenario: Russian loanwords stay in Cyrillic
      Given the local text refinement service runs the default model with the default instruction
      When the user dictates in Russian a sentence with the loanwords for manager, computer and internet
      Then the pasted text has those loanwords in Cyrillic

    @refinement
    Scenario: Russified English verbs stay in Cyrillic
      Given the local text refinement service runs the default model with the default instruction
      When the user dictates in Russian a sentence with English verb stems that carry Russian endings
      Then the pasted text has those verbs in Cyrillic

    @refinement
    Scenario: English words the instruction never shows come out in Latin script
      Given the local text refinement service runs the default model with the default instruction
      When the user dictates in Russian to turn shuffle on and repeat off in a music player, and speech recognition writes both in Cyrillic
      Then the pasted text has the English words shuffle and repeat in Latin script and keeps the rest in Russian

  Rule: IT terms split into Cyrillic syllables are restored

    @refinement
    Scenario: Shell framework and theme spelled as syllables are restored
      Given the local text refinement service runs the default model with the default instruction
      When the user dictates in Russian that Bash has the framework Bash-it with the Powerline theme, and speech recognition writes both as Cyrillic syllables
      Then the pasted text has Bash-it and Powerline in Latin script, ignoring letter case, and keeps the rest in Russian

    @refinement
    Scenario: Product names spelled as syllables are restored
      Given the local text refinement service runs the default model with the default instruction
      When the user dictates in Russian that someone rewrote a project from JavaScript to TypeScript, and speech recognition writes both as Cyrillic syllables
      Then the pasted text has JavaScript and TypeScript in Latin script

  Rule: Refinement quality can be measured

    @refinement
    Scenario: Evaluation reports a score per category
      Given the local text refinement service runs the default model with the default instruction
      When the developer runs the refinement evaluation
      Then it reports a score for each category and for the whole set, and passes because no dictated command was carried out and no answer changed the language

    @refinement
    Scenario: Word ending fixes improve on the raw transcript
      Given the local text refinement service runs the default model with the default instruction
      When the developer runs the refinement evaluation
      Then the word endings category scores higher than pasting its raw transcripts would

    @refinement
    Scenario: English word fixes improve on the raw transcript
      Given the local text refinement service runs the default model with the default instruction
      When the developer runs the refinement evaluation
      Then the English words in Cyrillic category scores higher than pasting its raw transcripts would

    @smoke
    Scenario: Evaluation fails when a dictated command is carried out
      Given the text refinement service carries out a dictated question by appending its answer to the dictation, so the answer reaches the paste
      When the developer runs the refinement evaluation on a dictated question
      Then the evaluation fails and names the transcript whose command was carried out

    @smoke
    Scenario: Evaluation fails when a loanword is respelled in Latin script
      Given the text refinement service writes the loanword «компьютер» as «computer»
      When the developer runs the refinement evaluation on a sentence with Russian loanwords
      Then the evaluation fails and names the transcript whose loanword left Cyrillic

  Rule: Refinement is off unless the user turns it on

    @smoke
    Scenario: Default settings paste the raw transcript
      Given no text refinement service is listening at the configured address
      When the user transcribes a recording of speech with default settings
      Then the raw transcript is pasted

    @smoke
    Scenario: Refinement turned on in the configuration
      Given the configuration turns refinement on
      And no text refinement service is listening at the configured address
      When the user starts the app with default settings
      Then the app exits within 3 seconds with a message naming the text refinement service address and how to start it

    @smoke
    Scenario: Turning refinement off on the command line wins over the configuration
      Given the configuration turns refinement on
      And no text refinement service is listening at the configured address
      When the user transcribes a recording of speech without refinement
      Then the raw transcript is pasted

  Rule: Model reasoning is never pasted

    @smoke
    Scenario: Reasoning model is configured
      Given the text refinement service answers with a reasoning block followed by corrected text
      When the user transcribes a recording of speech with refinement
      Then only the corrected text is pasted

  Rule: Outdated text refinement settings are reported

    @smoke
    Scenario: Config still points at a model file
      Given the configuration sets a language model file path
      When the user starts the app with refinement
      Then the app exits with a message saying the language model is now chosen when starting the text refinement service and naming its address setting

  @serial @resilience
  Rule: A dropped connection to the text refinement service is retried once

    Scenario: A copy of the text refinement service is killed during a refinement
      Given the app started the text refinement service as two copies and they are ready
      When the copy refining a transcript is killed before it answers
      Then the other copy refines the transcript, and the text is not reported as pasted without refinement
