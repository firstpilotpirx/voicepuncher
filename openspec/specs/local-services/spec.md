# local-services Specification

## Purpose
The local services the app needs — the speech recognition services for dictation and for commands, the text refinement service and the command spotting service — are started, watched, restarted and stopped: by the app itself in hotkey mode, or by the service scripts (`just start-model-servers`) when the app leaves them to the scripts.

## Requirements

### Requirement: A service that stops is started again
The service scripts SHALL keep each local service they started running: when the service process exits while it is meant to run, it SHALL be started again after a short pause, and the restart SHALL be written to that service's log after the output of the run that ended.

#### Scenario: The speech recognition service dies while voice control runs
- **GIVEN** the service scripts started the speech recognition service
- **WHEN** its process exits on its own
- **THEN** the service answers on its port again within 30 seconds, and its log keeps the output of the run that ended followed by a note of the restart

### Requirement: A stopped service stays stopped
The service scripts SHALL stop a service together with whatever keeps it running, so that a service the user stopped is not started again.

#### Scenario: The user stops the speech recognition service
- **GIVEN** the service scripts started the speech recognition service
- **WHEN** the user stops it with the service scripts
- **THEN** nothing answers on its port, still 5 seconds later, and no process of it is left

#### Scenario: The user stops a single speech recognition service after copies were left listed
- **GIVEN** a list of copies is left from an earlier start, and the service scripts started the speech recognition service as one copy
- **WHEN** the user stops it with the service scripts
- **THEN** nothing answers on its port, still 5 seconds later, and no process of it is left

### Requirement: A service that cannot start is reported, not retried forever
The service scripts SHALL report a service that exits before it ever became ready as a start failure that names the service and shows its last log lines, and SHALL leave no process of it behind.

#### Scenario: The speech recognition model is not a model
- **GIVEN** the speech recognition model file is not a model
- **WHEN** the service scripts start the speech recognition service
- **THEN** the start fails within 15 seconds naming the service and its log, and no process of it is left

### Requirement: The service scripts start the services named
The service scripts SHALL start only the services the user names, and all of them when none is named.

#### Scenario: Only the speech recognition service is started
- **GIVEN** no local service runs on the chosen ports
- **WHEN** the user starts only the speech recognition service for dictation with the service scripts
- **THEN** the speech recognition service for dictation answers and nothing answers on the ports of the speech recognition service for commands, the text refinement service and the command spotting service

### Requirement: The speech recognition service runs as several copies found from its address
The service scripts SHALL start the speech recognition service for dictation as several copies, three unless the user sets another number, on the ports a client finds from the service's address; a request SHALL go to a copy that is free, SHALL wait its turn while every copy is busy, first come first served, and no copy SHALL get two requests at once. A request SHALL keep its place in the queue for as long as it waits for a copy, also when its wait runs out and it tries again.

#### Scenario: Requests that arrive at once are shared by the copies
- **GIVEN** the service scripts started the speech recognition service for dictation without a number of copies
- **WHEN** several transcription requests arrive at once
- **THEN** every request gets its transcript, and each of the three copies transcribed some of them

#### Scenario: Requests wait their turn when every copy is busy
- **GIVEN** the speech recognition service for dictation runs as two copies, both busy with long speech
- **WHEN** two more requests arrive one after another
- **THEN** each starts on the first copy that frees, in the order they arrived, and no copy transcribes two requests at once

#### Scenario: A request whose wait ran out keeps its place
- **GIVEN** the only copy of the speech recognition service is busy for longer than a request waits for a copy
- **WHEN** two requests arrive one after another, and the copy frees just after the first one's wait ran out
- **THEN** the first request is transcribed on the copy before the second one

### Requirement: A copy that stops does not stop the service
The speech recognition service SHALL keep answering while one of its copies stops, and the stopped copy SHALL be started again.

#### Scenario: One copy of the speech recognition service dies
- **GIVEN** the service scripts started the speech recognition service as several copies
- **WHEN** one copy exits on its own
- **THEN** the service keeps answering transcription requests, and that copy answers again within 30 seconds

### Requirement: Stopping the speech recognition service stops every copy
The service scripts SHALL stop all copies of the speech recognition service, so that nothing of it keeps running.

#### Scenario: The user stops the speech recognition service that runs as several copies
- **GIVEN** the service scripts started the speech recognition service as several copies
- **WHEN** the user stops it with the service scripts
- **THEN** no copy of it is left, and nothing answers on any of its ports

### Requirement: Several copies that cannot start are reported and leave nothing behind
When a copy of the speech recognition service cannot start, the service scripts SHALL report the failure with that copy's log and SHALL stop the copies they already started.

#### Scenario: The second of several copies cannot start
- **GIVEN** the speech recognition service is to run as several copies, and its second copy cannot start
- **WHEN** the service scripts start the speech recognition service
- **THEN** the start fails naming the service and the log of the copy that failed, and the copy that started is not left

### Requirement: A service that stops answering is started again
The service scripts SHALL check every few seconds that each service they keep running still answers — the command spotting service on its connection, the text refinement service on its health check — and SHALL stop and start again a service that fails several checks in a row, writing the restart and its reason to that service's log. How often the scripts check SHALL be settable when they start, so a test proves the restart and the busy case through several checks without waiting the default cadence.

#### Scenario: The command spotting service hangs
- **GIVEN** the service scripts started the command spotting service
- **WHEN** its process stops answering without exiting
- **THEN** the service answers on its port again within 30 seconds, and its log notes that it was restarted because it stopped answering

#### Scenario: A busy service is not restarted
- **GIVEN** the service scripts started the command spotting service
- **WHEN** an app streams speech to it through several of the scripts' checks while the speech recognition service for commands answers its confirmations slowly
- **THEN** the service is not restarted

### Requirement: The command spotting service answers while its confirmations are slow
The command spotting service SHALL keep taking in audio, spotting commands and answering on its connection while a confirmation by the speech recognition service for commands is slow, failed or hung; a confirmation that is not answered within a few seconds SHALL be given up and logged, never ending the connection. How long the service waits for a confirmation SHALL be settable when it starts, so a test proves the give-up without waiting the default.

#### Scenario: One confirmation hangs
- **GIVEN** an app streams speech to the command spotting service
- **AND** the speech recognition service for commands never answers one confirmation
- **WHEN** the app keeps streaming past the wait for that confirmation
- **THEN** the connection stays open and answers the app throughout, and the hung confirmation is logged as given up

#### Scenario: Commands are spotted after a hung confirmation
- **GIVEN** an app streams speech to the command spotting service
- **AND** the speech recognition service for commands never answered one confirmation
- **WHEN** the app streams the command "press enter" after it
- **THEN** the service reports "press enter"

### Requirement: The command spotting service does not pile up confirmations
While a confirmation of a running utterance waits for the speech recognition service for commands, the command spotting service SHALL skip the next check of that utterance instead of queueing it, and SHALL try once more the confirmation that decides whether a whole utterance is a command when its first attempt is not answered in time.

#### Scenario: Checks of a long utterance are skipped while the service is slow
- **GIVEN** the speech recognition service for commands takes 4 seconds to answer every confirmation
- **WHEN** an app streams one utterance of 10 seconds
- **THEN** at most one check of that utterance waits at any time, and the checks that fall due meanwhile are skipped

#### Scenario: A command said alone is confirmed by the second attempt
- **GIVEN** the speech recognition service for commands does not answer the first confirmation in time and answers the second
- **WHEN** an app streams "press enter" said alone
- **THEN** the service reports "press enter"

### Requirement: A request given up by its client does not hold a copy
A client that gives up on a request SHALL leave its copy alone until the copy answers that request: the copy SHALL get no new request meanwhile, and the client's next request SHALL go to another copy or wait, so an abandoned request never delays the request after it on the same copy.

#### Scenario: The request after an abandoned one goes to another copy
- **GIVEN** the speech recognition service runs as two copies
- **AND** a client gave up on a request that one copy is still transcribing
- **WHEN** the client sends its next request
- **THEN** the other copy answers it, and the first copy gets a new request only after it answered the abandoned one

#### Scenario: An abandoned request is not transcribed
- **GIVEN** the speech recognition service runs as two copies, both busy with long speech
- **AND** a third request waits and its client gives up
- **WHEN** the copies become free
- **THEN** the abandoned request is not sent to any copy

### Requirement: Speech that is no command is confirmed quickly
The speech recognition service for commands SHALL answer the command spotting service's confirmation of any utterance up to 2.5 seconds in under 2 seconds when it is otherwise idle, also for speech in another language that makes speech recognition repeat itself, so a confirmation never waits out its deadline and never holds up the commands said after it. A confirmation the speech recognition service has not answered within 1 second of being sent SHALL be cut short: the command spotting service takes its utterance as no command, does not send it again, and the copy that held it takes the next confirmation at once.

#### Scenario: Russian speech that makes speech recognition repeat itself
- **GIVEN** voice control runs and the speech recognition service for commands is otherwise idle
- **WHEN** the user says Russian counting or filler words ("раз, два, три, четыре", "да, да, да, вот так") that the command spotting service sends for confirmation
- **THEN** every confirmation of that speech is answered in under 2 seconds

#### Scenario: A command right after such speech is carried out
- **GIVEN** voice control runs and the speech recognition service for commands is otherwise idle
- **WHEN** the user says that Russian speech and then "press enter"
- **THEN** the command spotting service reports "press enter"

#### Scenario: A confirmation still decoding after 1 second is cut short
- **GIVEN** the speech recognition service for commands runs as one copy, which keeps decoding an utterance for 1.5 seconds before it answers
- **WHEN** the command spotting service sends that utterance for confirmation
- **THEN** the confirmation ends in about 1 second with nothing heard, the utterance is not sent again, and the next confirmation is sent to that copy at once

### Requirement: Voice commands are confirmed by a speech recognition service of their own
The service scripts SHALL start, beside the speech recognition service for dictation, a speech recognition service for commands that recognizes speech with the Whisper small model, and the command spotting service they start SHALL send every confirmation to it, so that the larger dictation model never slows down the confirmation of a voice command.

#### Scenario: A command is confirmed by the speech recognition service for commands
- **GIVEN** the models that the service scripts download are on disk
- **AND** the user started all services with the service scripts
- **WHEN** an app streams "press enter" to the command spotting service
- **THEN** the command spotting service reports "press enter"
- **AND** the speech recognition service for commands answered its confirmation with the small model, and the speech recognition service for dictation got no request

### Requirement: Copies of a speech recognition service are found from its address
A client of a speech recognition service SHALL find the copies of the service from the service's address alone: when the service's own port accepts connections it is the one copy; otherwise every port among the four that follow its port plus ten thousand (minus ten thousand when those would pass the last port) that accepts a connection is a copy. A copy SHALL leave at its first refused connection and join, again or anew, within a second of accepting connections.

#### Scenario: Three copies are found from the service address
- **GIVEN** the service scripts started the speech recognition service for dictation as three copies
- **WHEN** a client connects to the service by its address
- **THEN** the client sends its requests to the three copies, and nothing listens on the service's own port

#### Scenario: A single copy on the service's own port is found
- **GIVEN** the service scripts started the speech recognition service for dictation as one copy
- **WHEN** a client connects to the service by its address
- **THEN** the client sends its requests to that copy on the service's own port

#### Scenario: A copy that came back is used again
- **GIVEN** a client of the speech recognition service, and one copy stopped accepting connections
- **WHEN** that copy accepts connections again
- **THEN** within a second the client sends requests to it again

### Requirement: A request whose copy died is retried on another copy
When a request cannot connect to the copy it took, the client SHALL mark that copy down and send the next attempt of the request to another copy that is up, within the request's own limits on attempts and time; when no attempt reaches a copy, the request SHALL fail naming the copies it tried.

#### Scenario: A copy dies as a request reaches it
- **GIVEN** the speech recognition service runs as several copies
- **AND** one copy exits as a request is about to reach it
- **WHEN** the request is sent
- **THEN** the request is answered by another copy, and the failed attempt on the dead copy is logged

#### Scenario: Every copy refuses the request
- **GIVEN** the speech recognition service runs as several copies, and none accepts connections
- **WHEN** a request is sent
- **THEN** the request fails once its attempts are used up, naming the copies it tried

### Requirement: A copy that stops answering is reported hung
A copy that has not answered a request within twice that request's time limit SHALL be logged as hung, naming the copy and the time waited, and SHALL get no new request until it accepts a connection and answers again.

#### Scenario: A copy never answers a request
- **GIVEN** the speech recognition service runs as several copies, and one copy stops answering without exiting
- **WHEN** a request waits on it past twice its time limit
- **THEN** that copy is logged as hung with the time waited, and the following requests go to the other copies

### Requirement: Clients of one process share the copies
Every client of one speech recognition service within one process SHALL share one view of its copies and one queue, and each process SHALL start its search for a free copy at a copy picked by its process id, so that two processes usually take different copies and requests sent at once from several clients or processes are spread over the copies rather than piled on the first one.

#### Scenario: Nine requests from one process spread over three copies
- **GIVEN** the speech recognition service for dictation runs as three copies
- **WHEN** nine clients of one process send a request each at once
- **THEN** each copy transcribes some of the requests, and no copy gets a second request before it answered the first

#### Scenario: Two processes start with different copies
- **GIVEN** the speech recognition service for dictation runs as three copies and is idle
- **AND** two processes whose process ids pick different copies
- **WHEN** each sends a request
- **THEN** the requests go to two different copies

### Requirement: The app starts the services it needs
When the app runs from the hotkey and manages its services, it SHALL start at launch every local service its configuration needs that does not already answer on its port, with the arguments the service scripts use, SHALL count a service ready once it answers within 60 seconds of its start, and SHALL start dictation only once every needed service is ready. Transcribing a file SHALL start no service.

#### Scenario: A cold launch starts the needed services
- **GIVEN** no local service runs and voice control is off
- **WHEN** the app launches
- **THEN** the speech recognition service for dictation is started as its copies and the text refinement service is started, unless refinement is off
- **AND** the speech recognition service for commands and the command spotting service are not started
- **AND** dictation starts once every started service answers

#### Scenario: Voice control needs the command services
- **GIVEN** no local service runs and voice control is on
- **WHEN** the app launches
- **THEN** the speech recognition service for commands is started as its copies and the command spotting service is started besides the others

#### Scenario: A real launch leaves no service behind at quit
- **GIVEN** no local service runs on the chosen ports and the models are on disk
- **WHEN** the app launches, its services become ready and the app is told to end
- **THEN** each started service answered on its port while the app ran
- **AND** no process of them is left after the app ended

### Requirement: A service that already answers is adopted
A needed service whose port already answers as that service SHALL be adopted: the app SHALL NOT start it, SHALL count it ready, SHALL keep watching it, and SHALL leave it running when the app quits.

#### Scenario: Services kept warm by the service scripts are adopted
- **GIVEN** the speech recognition service for dictation and the text refinement service already answer
- **WHEN** the app launches
- **THEN** the app starts no service and dictation starts
- **AND** the log says each service was adopted
- **AND** after the app quits both services still answer

### Requirement: An owned service that stops is started again
A service the app started SHALL be started again when its process exits, after 1 second, the pause doubling with every exit in a row up to 30 seconds and falling back to 1 second after a run of 30 seconds or more; a service with a health check (the text refinement and command spotting services; the whisper copies have none, as in the service scripts) that once answered and then fails three checks of it in a row SHALL be killed and started again the same way.

#### Scenario: An owned service exits
- **GIVEN** the app started the text refinement service and it is ready
- **WHEN** its process exits
- **THEN** the app starts it again after 1 second and it is ready again

#### Scenario: The pause grows while a service keeps exiting
- **GIVEN** the app started a service that answers and then exits within seconds of every start
- **WHEN** it exited three times in a row
- **THEN** the app waited 1, 2 and 4 seconds before the starts that followed

#### Scenario: An owned service that stops answering is restarted
- **GIVEN** the app started the text refinement service and it is ready
- **WHEN** it fails three checks in a row without exiting
- **THEN** the app kills it and starts it again

### Requirement: An owned service that cannot start is marked Failed
After five starts in a row of one service that each ended before the service answered — its process exited or 60 seconds passed — the app SHALL mark that service Failed with the reason and its log file, stop trying, and start it again only when the user asks to retry it.

#### Scenario: A service fails five starts
- **GIVEN** the text refinement service exits at every start
- **WHEN** the app started it five times
- **THEN** the service is Failed, naming its log file, and the app does not start it again
- **AND** dictation does not start

#### Scenario: Retry after a failure
- **GIVEN** the text refinement service is Failed
- **WHEN** the user asks to restart it and it now starts
- **THEN** the service is ready and dictation starts

### Requirement: Quit stops every owned service
When the app quits or is told to end, it SHALL stop every service it started — the command spotting service first, then the speech recognition copies, then the text refinement service — by asking it to end and killing it after 5 seconds, and SHALL leave every adopted service running.

#### Scenario: Quit with owned and adopted services
- **GIVEN** the app started the speech recognition service for dictation and adopted the text refinement service
- **WHEN** the user quits the app
- **THEN** no copy of the speech recognition service for dictation is left
- **AND** the text refinement service still answers

#### Scenario: A service that ignores the request to end is killed
- **GIVEN** the app started a service that does not end when asked
- **WHEN** the user quits the app
- **THEN** the service is killed 5 seconds after it was asked to end

### Requirement: An adopted service that goes away is reported and waited for
When an adopted service stops answering three checks in a row, the app SHALL report it as stopped outside the app, SHALL NOT start its own on that port, and SHALL count it adopted again once it answers again.

#### Scenario: The service scripts stop an adopted service
- **GIVEN** the app adopted the text refinement service
- **WHEN** the service stops answering
- **THEN** the service is shown as stopped outside the app and the app starts no text refinement service
- **AND** once the service answers again it is shown as adopted

### Requirement: Each started service logs to its own file
The app SHALL write the output and errors of every service it starts to a file of its own in the logs folder, named after the service's program and port as the service scripts name it, rotated by size like the app's log, and SHALL log each start, readiness, exit, restart, adoption and failure of a service in its own log.

#### Scenario: A started service writes its own log
- **GIVEN** the app started the text refinement service on its port
- **WHEN** the service writes to its output
- **THEN** the text is in the log file named after the text refinement program and that port in the logs folder
- **AND** the app's log says when the service became ready

### Requirement: The services can be left to the service scripts
When the configuration says the app does not manage its services, the app SHALL start, adopt, watch and stop no service, and SHALL connect to them as it does without this requirement.

#### Scenario: The app leaves the services alone
- **GIVEN** the configuration says the app does not manage its services
- **WHEN** the app launches
- **THEN** the app starts no service and connects to the services that answer

### Requirement: A missing program or model is named
A service whose program or model file cannot be found SHALL be Failed at once, without a start, with a reason naming what was looked for and where.

#### Scenario: The refinement model is not on disk
- **GIVEN** the text refinement model file is not on disk
- **WHEN** the app launches
- **THEN** the text refinement service is Failed with a reason naming the model file it looked for
