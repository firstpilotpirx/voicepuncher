# The service runtime on fake service programs (shell scripts started as the app starts its
# services), a probe told which ports answer and a clock the steps move; the copies on fake
# copies that answer over loopback. Scenarios watch real processes and ports, so @serial.
@local_services @component @serial
Feature: local-services

  Rule: A service that already answers is adopted

    Scenario: Services kept warm by the service scripts are adopted
      Given the service scripts keep the speech recognition service for dictation running
      When the app supervises the services it needs
      Then the app starts no service and counts the speech recognition service for dictation adopted and ready
      And after the app quits the service the scripts run still runs

  Rule: An adopted service that goes away is reported and waited for

    Scenario: The service scripts stop an adopted service
      Given the app adopted the speech recognition service for dictation
      When the service the scripts run stops answering
      Then the service is shown as stopped outside the app and the app starts no service
      And once the service answers again it is shown as adopted

  Rule: An owned service that cannot start is marked Failed

    Scenario: A service fails five starts
      Given the command spotting service exits at every start
      When the app started it five times
      Then the service is Failed, naming its log file, and the app does not start it again

    Scenario: Retry after a failure
      Given the command spotting service is Failed
      When the user asks to restart it and it now starts
      Then the command spotting service is ready

  Rule: Quit stops every owned service

    Scenario: Quit with owned and adopted services
      Given the app started the speech recognition service for dictation and adopted the speech recognition service for commands
      When the user quits the app with its services
      Then no process of the speech recognition service for dictation is left
      And the speech recognition service for commands still runs

    Scenario: A service that ignores the request to end is killed
      Given the app started a service that does not end when asked
      When the user quits the app with its services
      Then the service is killed 5 seconds after it was asked to end

  Rule: Each started service logs to its own file

    Scenario: A started service writes its own log
      Given the app started the command spotting service on its port
      When the service writes to its output
      Then the text is in the log file named after the command spotting program and that port in the logs folder

    Scenario: A service keeps its log after the app that started it quits
      Given the app started the command spotting service on its port
      When the app quits without stopping it and the service writes to its output
      Then the text is in the service's log file

  Rule: The services can be left to the service scripts

    Scenario: The app leaves the services alone
      Given the configuration says the app does not manage its services
      And the speech recognition service answers
      When the app launches with that configuration
      Then the app starts no service and connects to the services that answer

  Rule: A bundled app starts its services from its own engines

    Scenario: The bundle's engine comes before $PATH
      Given the app runs from a bundle whose engine folders hold every engine, and $PATH holds them too
      When the services it needs are looked up
      Then each service's program is the one in its engine folder inside the bundle

    Scenario: An app outside a bundle uses $PATH
      Given the app runs from a build folder and $PATH holds every engine
      When the services it needs are looked up
      Then each service's program is the one on $PATH

  Rule: The engine in use is logged

    Scenario: A bundled engine is logged with its version
      Given the app runs from a bundle whose speech recognition engine folder has the version file 1.9.5
      When the services it needs are looked up
      Then the speech recognition service is named as running whisper-server 1.9.5 from the bundle

    Scenario: A Homebrew engine is logged with its version
      Given the app runs outside a bundle and $PATH leads to whisper-server in Homebrew's folder of version 1.9.5
      When the services it needs are looked up
      Then the speech recognition service is named as running whisper-server 1.9.5 from $PATH

  Rule: A missing service program or model is named

    Scenario: The command model is not on disk
      Given the variable for the speech recognition model for commands names a file that is not on disk
      When the services it needs are looked up and supervised
      Then the speech recognition service for commands is Failed with a reason naming the model file it looked for
      And the app started no service

  Rule: The app and the service scripts start a service alike

    Scenario: Of two keyword spotting models the newest is used
      Given two keyword spotting model folders in the models folder, the older one first by name
      When the services it needs are looked up
      Then the command spotting service is started with the newer folder

  Rule: The app runs the command spotting service as copies

    Scenario: Voice control starts two copies of the command spotting service
      Given no local service runs, voice control is on and the number of its copies is not configured
      When the services it needs are looked up
      Then the command spotting service is started as two copies on its copy ports, and nothing on its own port

    Scenario: One configured copy runs on the service's own port
      Given voice control is on and the command spotting service is configured as one copy
      When the services it needs are looked up
      Then the command spotting service is started once, on its own port

    Scenario: The copy ports of the services never overlap
      Given every service on its default address
      When the copy ports of each service are worked out
      Then no port is a copy port of two services

  Rule: A client of the copies starts without waiting for their discovery

    Scenario: A client is created while the copies are slow to probe
      Given probing the copies of a service takes a while
      When a client of those copies is created
      Then it returns at once, and its first request goes to a copy that answers

  Rule: A request whose copy died is retried on another copy

    Scenario: A copy dies as a request reaches it
      Given the speech recognition service runs as two copies, and the copy tried first exits as the request is about to reach it
      When a request is sent to the copies
      Then the request is answered by the other copy, and the failed attempt on the dead copy is logged

    Scenario: Every copy refuses the request
      Given the speech recognition service runs as two copies, and none accepts connections
      When a request is sent to the copies
      Then the request fails once its attempts are used up, naming the copies it tried

  Rule: A copy that stops answering is reported hung

    Scenario: A copy never answers a request
      Given the speech recognition service runs as two copies, and one copy stops answering without exiting
      When a request waits on that copy past twice its time limit
      Then that copy is logged as hung with the time waited, and the following requests go to the other copy

  Rule: Clients of one process share the copies

    Scenario: Nine requests from one process spread over three copies
      Given the speech recognition service for dictation runs as three copies
      When nine clients of one process send a request each at once
      Then each copy transcribes some of the requests, and no copy gets a second request before it answered the first

    Scenario: Two processes start with different copies
      Given the speech recognition service for dictation runs as three idle copies, and two processes whose process ids pick different copies
      When each process sends a request
      Then the requests go to two different copies

  Rule: A hung confirmation makes the command spotting service unhealthy

    Scenario: The app leaves a stalled service
      Given the app listens through a command spotting service
      When the command spotting service reports that it stalled
      Then the stalled service is counted as lost and the app connects to it again
