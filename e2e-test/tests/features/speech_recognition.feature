@speech_recognition @e2e
Feature: speech-recognition

  Rule: Transcription through a local speech recognition service

    @smoke
    Scenario: App starts without a local model file
      Given the speech recognition service is running
      And no speech recognition model is on disk for the app
      When the user starts the app
      Then the app does not exit with a startup failure

  Rule: Speech recognition service stays on this machine

    @smoke
    Scenario: Remote service address is rejected
      Given the speech recognition service address is on another host
      When the user starts the app
      Then the app exits before recording anything and explains that only a local service is allowed

  Rule: Unavailable speech recognition service is reported at startup

    @smoke
    Scenario: Service is not running at startup
      Given no speech recognition service is listening at the configured address
      When the user starts the app
      Then the app exits within 3 seconds with a message naming the address and how to start the service

  Rule: Outdated speech recognition settings are reported

    @smoke
    Scenario: Config still points at a model file
      Given the configuration sets a speech recognition model file path
      When the user starts the app
      Then the app exits with a message saying the model is now chosen when starting the service and naming the service address setting

    @smoke
    Scenario: Config still sets a fixed silence level
      Given the configuration sets a fixed silence level for chunking
      When the user starts the app
      Then the app exits with a message saying pauses now follow the recording's background noise and the setting can be deleted

  Rule: Known recognition hallucinations are discarded

    @smoke
    Scenario Outline: Silence produces a lone "you"
      Given a recording of silence lasting <seconds> seconds
      And the service detects the spoken language automatically
      When the user transcribes the recording file without refinement
      Then no text is produced

      Examples:
        | seconds |
        | 1       |
        | 10      |
        | 60      |

  Rule: Long recordings are transcribed in chunks

    @smoke
    Scenario Outline: Recording file longer than the chunk threshold
      Given a recording file of <language> speech longer than the chunk threshold
      When the user transcribes the recording file without refinement
      Then the recording is transcribed in chunks
      And the printed text repeats no phrase of the speech
      And the printed text misses at most one spoken word in ten

      Examples:
        | language |
        | Russian  |
        | English  |

    @seams
    Scenario Outline: Long speech under difficult recording conditions
      Given a recording file of <language> speech longer than the chunk threshold, <condition>
      When the user transcribes the recording file without refinement
      Then the printed text repeats no phrase of the speech
      And the printed text misses at most one spoken word in ten

      Examples:
        | language | condition                     |
        | Russian  | spoken at a normal pace       |
        | Russian  | spoken slowly                 |
        | Russian  | spoken fast                   |
        | Russian  | with pauses between sentences |
        | Russian  | with pauses between words     |
        | Russian  | with low background noise     |
        | Russian  | with high background noise    |
        | Russian  | through a phone-quality line  |
        | Russian  | spoken quietly                |
        | Russian  | clipped                       |
        | Russian  | with short audio dropouts     |
        | English  | spoken at a normal pace       |
        | English  | spoken slowly                 |
        | English  | spoken fast                   |
        | English  | with pauses between sentences |
        | English  | with pauses between words     |
        | English  | with low background noise     |
        | English  | with high background noise    |
        | English  | through a phone-quality line  |
        | English  | spoken quietly                |
        | English  | clipped                       |
        | English  | with short audio dropouts     |

  Rule: Speech after a pause is transcribed

    @seams
    Scenario Outline: Speech with pauses between words or sentences
      Given a recording file of <language> speech longer than the chunk threshold, <condition>
      When the user transcribes the recording file without refinement
      Then no run of more than five consecutive spoken words is missing from the printed text

      Examples:
        | language | condition                     |
        | Russian  | with pauses between words     |
        | Russian  | with pauses between sentences |
        | English  | with pauses between words     |
        | English  | with pauses between sentences |

  Rule: Audio without speech never stops the speech recognition service

    @smoke
    Scenario: Command confirmation of an utterance without speech
      Given the service scripts started the speech recognition service
      When the command spotting service asks it to confirm an utterance that holds no speech
      Then no words are heard and the service answers the next request
