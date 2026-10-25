# local-services Specification

## Purpose
The local services the app needs — the speech recognition services for dictation and for commands and the command spotting service — are started, watched, restarted and stopped: by the app itself in hotkey mode, or by the service scripts (`just start-model-servers`) when the app leaves them to the scripts.

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
- **THEN** the speech recognition service for dictation answers and nothing answers on the ports of the speech recognition service for commands and the command spotting service

#### Scenario: The scripts know no text refinement service
- **GIVEN** no local service runs on the chosen ports
- **WHEN** the user starts every service with the service scripts
- **THEN** the speech recognition services for dictation and for commands and the command spotting service answer
- **AND** no text refinement service is started and nothing answers on its former port 8080

### Requirement: The speech recognition service runs as several copies found from its address
The service scripts SHALL start the speech recognition service for dictation as several copies, three unless the user sets another number, on the ports a client finds from the service's address; a request SHALL go to a copy that is free, SHALL wait its turn while every copy is busy, first come first served, and no copy SHALL get two requests at once. A request SHALL keep its place in the queue for as long as it waits for a copy, also when its wait runs out and it tries again. The sharing of requests that arrive at once SHALL hold while the machine is busy with another run: a check of it waits for every transcript as long as the queued transcriptions take on a loaded machine, and when a request still gets no transcript, the failure SHALL name that request, the copies it waited on and what the machine was busy with.

#### Scenario: Requests that arrive at once are shared by the copies
- **GIVEN** the service scripts started the speech recognition service for dictation without a number of copies
- **WHEN** several transcription requests arrive at once
- **THEN** every request gets its transcript, and each of the three copies transcribed some of them

#### Scenario: Requests that arrive at once are shared by the copies while the machine is busy
- **GIVEN** the service scripts started the speech recognition service for dictation without a number of copies, and another run keeps the machine's processor busy
- **WHEN** several transcription requests arrive at once
- **THEN** every request gets its transcript within the time the queued transcriptions take on the busy machine, and each of the three copies transcribed some of them

#### Scenario: A request without a transcript names the machine's load
- **GIVEN** the service scripts started the speech recognition service for dictation without a number of copies
- **WHEN** several transcription requests arrive at once and one of them gets no transcript in time
- **THEN** the failure names that request, the copies it waited on and the busiest processes on the machine

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
The service scripts SHALL check every few seconds that each service they keep running still answers — the command spotting service on its connection — and SHALL stop and start again a service that fails several checks in a row, writing the restart and its reason to that service's log. How often the scripts check SHALL be settable when they start, so a test proves the restart and the busy case through several checks without waiting the default cadence.

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
The speech recognition service for commands SHALL answer the command spotting service's confirmation of any utterance up to 2.5 seconds in under 2 seconds when it is otherwise idle, also for speech in another language that makes speech recognition repeat itself, so a confirmation never waits out its deadline and never holds up the commands said after it. A confirmation the speech recognition service has not answered within its decoding budget SHALL be cut short: the command spotting service takes its utterance as no command, does not send it again, and the copy that held it takes the next confirmation at once. The decoding budget SHALL follow the pace of the service's recent answers to ordinary speech: 1 second when they came back quickly, longer when the last few took long, as under machine load, and never past the confirmation's deadline, so speech that decodes slowly because the machine is busy is heard in full while speech that decodes long because speech recognition repeats itself is still cut short. A confirmation cut short SHALL lengthen the budget to 1.5 seconds and no further, however many are cut short and whatever budget they were cut at, and an answer of speech recognition stuck repeating SHALL NOT lengthen the budget, so speech stuck repeating never raises the budget past the time a confirmation of it may take.

#### Scenario: Russian speech that makes speech recognition repeat itself
- **GIVEN** voice control runs and the speech recognition service for commands is otherwise idle
- **WHEN** the user says Russian counting or filler words ("раз, два, три, четыре", "да, да, да, вот так") that the command spotting service sends for confirmation
- **THEN** every confirmation of that speech is answered in under 2 seconds

#### Scenario: A command right after such speech is carried out
- **GIVEN** voice control runs and the speech recognition service for commands is otherwise idle
- **WHEN** the user says that Russian speech and then "press enter"
- **THEN** the command spotting service reports "press enter"

#### Scenario: A confirmation still decoding after 1 second is cut short
- **GIVEN** the speech recognition service for commands runs as one copy, which answered its recent confirmations quickly and keeps decoding an utterance for 1.5 seconds before it answers
- **WHEN** the command spotting service sends that utterance for confirmation
- **THEN** the confirmation ends in about 1 second with nothing heard, the utterance is not sent again, and the next confirmation is sent to that copy at once

#### Scenario: A confirmation decoding slowly under load is heard in full
- **GIVEN** the speech recognition service for commands runs as one copy, which took close to a second for each of its recent confirmations, as under machine load, and keeps decoding "press enter" for 1.5 seconds before it answers
- **WHEN** the command spotting service sends that utterance for confirmation
- **THEN** the confirmation is answered with "press enter", not cut short

#### Scenario: The budget never passes the deadline
- **GIVEN** the speech recognition service for commands runs as one copy, which took longer than half the confirmation deadline for each of its recent confirmations
- **WHEN** the command spotting service sends an utterance the copy keeps decoding past the deadline
- **THEN** the confirmation ends at the deadline as a failed confirmation, not later and never as an utterance heard as no command

#### Scenario: Speech cut short does not lengthen the budget past 1.5 seconds
- **GIVEN** the speech recognition service for commands runs as one copy, which answered its recent confirmations quickly, and the last confirmation was cut short after 1 second
- **WHEN** the command spotting service sends two more utterances the copy keeps decoding for 2.5 seconds each
- **THEN** each of them is cut short after about 1.5 seconds

#### Scenario: An answer stuck repeating does not lengthen the budget
- **GIVEN** the speech recognition service for commands runs as one copy, which took close to a second for each of its recent confirmations, as under machine load, and then answered one confirmation after 1.6 seconds with speech recognition stuck repeating one word
- **WHEN** the command spotting service sends an utterance the copy keeps decoding for 2.5 seconds
- **THEN** that confirmation is cut short after about 2 seconds, the budget the ordinary answers give, not at the deadline

### Requirement: Voice commands are confirmed by a speech recognition service of their own
The service scripts SHALL start, beside the speech recognition service for dictation, a speech recognition service for commands that recognizes speech with the Whisper small model, and the command spotting service they start SHALL send every confirmation to it, so that the larger dictation model never slows down the confirmation of a voice command.

#### Scenario: A command is confirmed by the speech recognition service for commands
- **GIVEN** the models that the service scripts download are on disk
- **AND** the user started all services with the service scripts
- **WHEN** an app streams "press enter" to the command spotting service
- **THEN** the command spotting service reports "press enter"
- **AND** the speech recognition service for commands answered its confirmation with the small model, and the speech recognition service for dictation got no request

### Requirement: Copies of a speech recognition service are found from its address
A client of a speech recognition service SHALL find the copies of the service from the service's address alone: when the service's own port answers as a speech recognition service it is the one copy; otherwise every port among the four that follow its port plus ten thousand (minus ten thousand when those would pass the last port) that answers as a speech recognition service is a copy. A port answers as a speech recognition service when it answers the request for its start page (`GET /`) with a page that names it a whisper-server; a program that accepts connections on a port but does not answer so SHALL never receive a transcription request, whether it holds the service's own port or a copy's. A copy SHALL leave at its first refused connection to a request, and at a probe only when two attempts in a row get no connection, so one probe that is slow under machine load does not drop a copy that accepts connections; a copy SHALL join, again or anew, within a second of answering as a speech recognition service.

#### Scenario: Three copies are found from the service address
- **GIVEN** the service scripts started the speech recognition service for dictation as three copies
- **WHEN** a client connects to the service by its address
- **THEN** the client sends its requests to the three copies, and nothing listens on the service's own port

#### Scenario: A single copy on the service's own port is found
- **GIVEN** the service scripts started the speech recognition service for dictation as one copy
- **WHEN** a client connects to the service by its address
- **THEN** the client sends its requests to that copy on the service's own port

#### Scenario: Another program on the service's own port is passed over
- **GIVEN** the speech recognition service for dictation runs as three copies, and another program that is no speech recognition service listens on the service's own port
- **WHEN** a client transcribes a dictation through the service's address
- **THEN** the client sends its requests to the three copies, the dictation is transcribed, and the other program receives no transcription request

#### Scenario: Another program on a copy's port is passed over
- **GIVEN** the speech recognition service for dictation runs as two copies, and another program that is no speech recognition service listens on the third copy's port
- **WHEN** a client transcribes a dictation through the service's address
- **THEN** the client sends its requests to the two copies only

#### Scenario: A copy that came back is used again
- **GIVEN** a client of the speech recognition service, and one copy stopped accepting connections
- **WHEN** that copy accepts connections again
- **THEN** within a second the client sends requests to it again

#### Scenario: A copy missed by one slow probe stays
- **GIVEN** a client of the speech recognition service running as several copies, and the machine is so busy that one probe of a copy gets no connection in time while the copy still accepts connections
- **WHEN** the client probes the copies
- **THEN** the client probes that copy once more at once, and keeps sending requests to it

#### Scenario: A copy missed by two probes in a row leaves
- **GIVEN** a client of the speech recognition service running as several copies
- **WHEN** a copy gets no connection at two probe attempts in a row
- **THEN** the client sends no request to that copy until it accepts connections again

#### Scenario: A retry after a hung request reaches the other copy
- **GIVEN** the speech recognition service for dictation runs as two copies, one of which never answers, and both stay found by the probes (a probe that misses one once keeps it, as the scenario "A copy missed by one slow probe stays" proves)
- **WHEN** a transcription request hangs on the silent copy and is retried
- **THEN** the retry is answered by the other copy, and both copies received one request

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
When the app runs from the hotkey and manages its services, it SHALL start at launch every local service its configuration needs that does not already answer on its port, with the arguments the service scripts use, SHALL count a service ready once it answers within 60 seconds of its start, and SHALL start dictation only once every needed service is ready. Transcribing a file SHALL start no service. The app SHALL never start, adopt or watch a text refinement service.

#### Scenario: A cold launch starts the needed services
- **GIVEN** no local service runs and voice control is off
- **WHEN** the app launches
- **THEN** the speech recognition service for dictation is started as its copies
- **AND** the speech recognition service for commands and the command spotting service are not started
- **AND** no text refinement service is started
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
- **GIVEN** the speech recognition service for dictation already answers
- **WHEN** the app launches
- **THEN** the app starts no service and dictation starts
- **AND** the log says the service was adopted
- **AND** after the app quits the service still answers

### Requirement: An owned service that stops is started again
A service the app started SHALL be started again when its process exits, after 1 second, the pause doubling with every exit in a row up to 30 seconds and falling back to 1 second after a run of 30 seconds or more; a service or copy that once answered its health check and then fails three checks of it in a row SHALL be killed and started again the same way. Every service the app starts is health-checked, each copy of a speech recognition service on its own.

#### Scenario: An owned service exits
- **GIVEN** the app started the command spotting service and it is ready
- **WHEN** its process exits
- **THEN** the app starts it again after 1 second and it is ready again

#### Scenario: The pause grows while a service keeps exiting
- **GIVEN** the app started a service that answers and then exits within seconds of every start
- **WHEN** it exited three times in a row
- **THEN** the app waited 1, 2 and 4 seconds before the starts that followed

#### Scenario: An owned service that stops answering is restarted
- **GIVEN** the app started the command spotting service and it is ready
- **WHEN** it fails three checks in a row without exiting
- **THEN** the app kills it and starts it again

#### Scenario: An owned speech recognition copy that stops answering is restarted
- **GIVEN** the app started the speech recognition service for dictation and its copies are ready
- **WHEN** one copy stops answering without exiting
- **THEN** within 30 seconds the app has killed that copy and started it again, and it answers on its port
- **AND** the other copies were never restarted

### Requirement: An owned service that cannot start is marked Failed
After five starts in a row of one service that each ended before the service answered — its process exited or 60 seconds passed — the app SHALL mark that service Failed with the reason and its log file, stop trying, and start it again only when the user asks to retry it.

#### Scenario: A service fails five starts
- **GIVEN** the command spotting service exits at every start
- **WHEN** the app started it five times
- **THEN** the service is Failed, naming its log file, and the app does not start it again
- **AND** dictation does not start

#### Scenario: Retry after a failure
- **GIVEN** the command spotting service is Failed
- **WHEN** the user asks to restart it and it now starts
- **THEN** the service is ready and dictation starts

### Requirement: Quit stops every owned service
When the app quits or is told to end — Quit in its menu, a quit from the system (logout, shutdown, Activity Monitor's Quit, any quit Apple Event), a termination or an interrupt signal — it SHALL log the quit, naming a quit from the system or the signal it got, and SHALL stop every service it started — the command spotting service first, then the speech recognition copies — by asking it to end and killing it after 5 seconds, and SHALL leave every adopted service running.

#### Scenario: Quit with owned and adopted services
- **GIVEN** the app started the speech recognition service for dictation and adopted the speech recognition service for commands
- **WHEN** the user quits the app
- **THEN** no copy of the speech recognition service for dictation is left
- **AND** the speech recognition service for commands still answers

#### Scenario: A service that ignores the request to end is killed
- **GIVEN** the app started a service that does not end when asked
- **WHEN** the user quits the app
- **THEN** the service is killed 5 seconds after it was asked to end

#### Scenario: A quit from the system stops the owned services
- **GIVEN** the app started the speech recognition service for dictation and it is ready
- **WHEN** the system tells the app to quit
- **THEN** the app's log names the quit from the system
- **AND** no process of the speech recognition service for dictation is left after the app ended

### Requirement: An adopted service that goes away is reported and waited for
When an adopted service stops answering three checks in a row, the app SHALL report it as stopped outside the app, SHALL NOT start its own on that port, and SHALL count it adopted again once it answers again.

#### Scenario: The service scripts stop an adopted service
- **GIVEN** the app adopted the speech recognition service for dictation
- **WHEN** the service stops answering
- **THEN** the service is shown as stopped outside the app and the app starts no speech recognition service for dictation
- **AND** once the service answers again it is shown as adopted

### Requirement: Each started service logs to its own file
The app SHALL write the output and errors of every service it starts to a file of its own in the logs folder, named after the service's program and port as the service scripts name it, and SHALL log each start, readiness, exit, restart, adoption and failure of a service in its own log. The service SHALL write into that file itself, not through the app, so its log goes on when the app that started it quits and a later app adopts it. The file SHALL be rotated by size when a service starts, like the app's log.

#### Scenario: A started service writes its own log
- **GIVEN** the app started the command spotting service on its port
- **WHEN** the service writes to its output
- **THEN** the text is in the log file named after the command spotting program and that port in the logs folder
- **AND** the app's log says when the service became ready

#### Scenario: A service keeps its log after the app that started it quits
- **GIVEN** the app started a service
- **WHEN** the app quits without stopping it and the service writes to its output
- **THEN** the text is in the service's log file

### Requirement: The services can be left to the service scripts
When the configuration says the app does not manage its services, the app SHALL start, adopt, watch and stop no service, and SHALL connect to them as it does without this requirement.

#### Scenario: The app leaves the services alone
- **GIVEN** the configuration says the app does not manage its services
- **WHEN** the app launches
- **THEN** the app starts no service and connects to the services that answer

### Requirement: A bundled app starts its services from its own engines
When the app runs from its application bundle, it SHALL start each service from the engine folder of that service inside the bundle before looking on `$PATH`; a program named by the environment SHALL still come first, and an app that does not run from a bundle SHALL find its engines on `$PATH` as before.

#### Scenario: The bundle's engine comes before `$PATH`
- **GIVEN** the app runs from a bundle whose engine folders hold every engine, and `$PATH` holds them too
- **WHEN** the services it needs are looked up
- **THEN** each service's program is the one in its engine folder inside the bundle

#### Scenario: An app outside a bundle uses `$PATH`
- **GIVEN** the app runs from a build folder and `$PATH` holds every engine
- **WHEN** the services it needs are looked up
- **THEN** each service's program is the one on `$PATH`

### Requirement: The engine in use is logged
At every launch that manages services, the app SHALL log for each service the engine it runs, where the engine came from — the bundle, a program the environment names, `$PATH` or the build folder — and the engine's version when it is known: the version file of a bundled engine, or the version folder of a Homebrew installation.

#### Scenario: A bundled engine is logged with its version
- **GIVEN** the app runs from a bundle whose speech recognition engine folder has the version file 1.9.5
- **WHEN** the app launches and looks up its services
- **THEN** its log says the speech recognition service runs whisper-server 1.9.5 from the bundle

#### Scenario: A Homebrew engine is logged with its version
- **GIVEN** the app runs outside a bundle and `$PATH` leads to whisper-server in Homebrew's folder of version 1.9.5
- **WHEN** the app launches and looks up its services
- **THEN** its log says the speech recognition service runs whisper-server 1.9.5 from `$PATH`

### Requirement: A hung confirmation makes the command spotting service unhealthy
The command spotting service SHALL count a confirmation that runs longer than its attempts may take, and at least 10 seconds, as hung: its health check SHALL answer unavailable naming the connection, and it SHALL tell the app on that connection, which counts the service as lost and reaches for another copy.

#### Scenario: A confirmation thread that hangs is seen
- **GIVEN** an app streams speech to the command spotting service
- **AND** one confirmation hangs inside the service past its attempts
- **WHEN** the health of the service is checked
- **THEN** it answers unavailable naming the connection, and the app was told the service stalled

#### Scenario: The app leaves a stalled service
- **GIVEN** listening is on
- **WHEN** the command spotting service reports that it stalled
- **THEN** the service is counted as lost and the app reaches for it again

### Requirement: Confirmations waiting in the command spotting service are bounded
The command spotting service SHALL keep at most a few confirmations waiting per confirmation thread of a connection while the confirmation that thread runs is stalled; a confirmation asked past that SHALL fail at once and be logged, never queue without bound. A thread that keeps answering, however slowly, SHALL queue every confirmation asked of it.

#### Scenario: A confirmation past the bound fails at once
- **GIVEN** the speech recognition service for commands answers nothing, the confirmation a thread runs has run past its attempts, and the thread already holds its bound of waiting confirmations
- **WHEN** one more confirmation is asked on that thread
- **THEN** it fails at once and the failure is logged

#### Scenario: A slow confirmation thread keeps every confirmation
- **GIVEN** the speech recognition service for commands answers every confirmation, late
- **WHEN** more confirmations than the bound are asked on one thread at once
- **THEN** every one of them is answered with what was heard

### Requirement: A client of the copies starts without waiting for their discovery
Creating a client of a service's copies SHALL return at once; the copies SHALL be probed in the background, and the client's first request SHALL wait for the first probe to finish, so it never takes a copy that answers for one that does not.

#### Scenario: A client is created while the copies are slow to probe
- **GIVEN** probing the copies of a service takes a second
- **WHEN** a client of those copies is created
- **THEN** it returns at once, and its first request goes to a copy that answers

### Requirement: The app and the service scripts start a service alike
The app SHALL start each local service with the same arguments as the service scripts, and SHALL pick a model the same way the service scripts do: of several matching model files or folders, the newest. A difference between the two SHALL fail the tests.

#### Scenario: The app and the scripts start the speech recognition service with the same arguments
- **GIVEN** the service scripts and the app's own list of service command lines
- **WHEN** the arguments of the speech recognition services and the command spotting service are compared
- **THEN** each service gets the same arguments in the same order from both

#### Scenario: A changed argument in only one place is caught
- **GIVEN** a service argument that the service scripts set and the app does not
- **WHEN** the tests run
- **THEN** they fail, naming the service whose arguments differ

#### Scenario: Of two keyword spotting models the newest is used
- **GIVEN** two keyword spotting model folders in the models folder, the older one first by name
- **WHEN** the services the configuration needs are looked up
- **THEN** the command spotting service is started with the newer folder

### Requirement: A copy of a speech recognition service is ready once it has transcribed speech
The service scripts and the app SHALL count a copy of a speech recognition service they start as ready only after it answered the transcription of a short utterance, and SHALL report the copy ready together with how long that first transcription took, so the first request a client sends a copy never waits for the copy's first decode. A copy the app starts again after it stopped SHALL be warmed the same way before it counts ready; a copy the scripts' supervisor starts again after a crash is warmed by its first request instead. The checks of a running copy SHALL send it no speech.

#### Scenario: The service scripts report a copy ready after its first transcription
- **GIVEN** no local service runs on the chosen ports
- **WHEN** the service scripts start the speech recognition service for commands
- **THEN** each copy is reported ready with the time of its first transcription, and the copy's log shows that transcription before the report

#### Scenario: The app counts a started copy ready after its first transcription
- **GIVEN** the app started a copy of a speech recognition service
- **WHEN** the copy answers on its port but has not transcribed the utterance the app sent it yet
- **THEN** the app does not count the copy ready until the copy transcribed that utterance

#### Scenario: A copy that cannot transcribe in time is not ready
- **GIVEN** the app started a copy of a speech recognition service that answers on its port
- **WHEN** the copy does not answer the transcription of the utterance within the time a start may take
- **THEN** the copy counts as not ready in time and is started again, as a copy that never answered

#### Scenario: The check of a ready copy sends it no speech
- **GIVEN** a copy the app started is ready
- **WHEN** the app checks the copy while it runs
- **THEN** the check asks only whether the copy answers, and sends it no speech to transcribe

### Requirement: A missing service program or model is named
A service whose program, or a model file named by its variable, cannot be found SHALL be Failed at once, without a start, with a reason naming what was looked for and where. A model of the catalog missing from the models folder does not reach the services: the runtime waits for models first.

#### Scenario: The command model is not on disk
- **GIVEN** the variable for the speech recognition model for commands names a file that is not on disk
- **WHEN** the services the configuration needs are looked up
- **THEN** the speech recognition service for commands is Failed with a reason naming the model file it looked for

### Requirement: The app runs the command spotting service as copies
When the app manages its services, it SHALL start the command spotting service as the configured number of copies, 1 to 4, 2 by default, found from the service's address as the copies of a speech recognition service are, on copy ports apart from those of every other service. A service already answering on its own port or its copy ports SHALL be adopted as it runs.

#### Scenario: Voice control starts two copies of the command spotting service
- **GIVEN** no local service runs, voice control is on and the number of its copies is not configured
- **WHEN** the app launches
- **THEN** the command spotting service is started as two copies on its copy ports, and nothing is started on its own port

#### Scenario: One configured copy runs on the service's own port
- **GIVEN** voice control is on and the command spotting service is configured as one copy
- **WHEN** the app launches
- **THEN** the command spotting service is started once, on its own port

#### Scenario: The copy ports of the services never overlap
- **GIVEN** every service on its default address
- **WHEN** the app works out the copy ports of each
- **THEN** no port is a copy port of two services

#### Scenario: A command spotting service the scripts run is adopted
- **GIVEN** the service scripts run the command spotting service on its own port
- **WHEN** the app launches with voice control on
- **THEN** the app adopts it and starts no copy of it

### Requirement: Another program on a speech recognition port is named in the log
When another program that is no speech recognition service listens on the own port of a speech recognition service, or on one of its copies' ports, the client SHALL log a warning once per port, naming the port, the program's name and its process id, as long as the program stays there.

#### Scenario: A program on the dictation port is logged once
- **GIVEN** another program that is no speech recognition service listens on the own port of the speech recognition service for dictation
- **WHEN** a client of the service transcribes two dictations
- **THEN** the log holds one warning naming that port, the program's name and its process id
