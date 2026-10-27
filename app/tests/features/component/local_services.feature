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

  Rule: An adopted service that stops is replaced by the app's own

    Scenario: An adopted command spotting copy that stops is replaced
      Given the app adopted both copies of the command spotting service
      When the process of one adopted copy ends
      Then within 30 seconds the app has started its own copy on that port and it answers
      And the log names the service, the copy, the port and that it stopped
      And the service is shown ready again

    Scenario: An adopted speech recognition copy that stops is replaced
      Given the app adopted both copies of the speech recognition service for dictation
      When the process of one adopted copy ends
      Then within 30 seconds the app has started its own copy on that port and it answers
      And the other copy was never restarted

    Scenario: A replaced copy is stopped at quit
      Given the app replaced an adopted copy of the speech recognition service for dictation that stopped
      When the user quits the app with its services
      Then no process of the replaced copy is left and the copy still adopted keeps running

  Rule: An adopted service that hangs is replaced by the app's own

    Scenario: A hung adopted copy is killed and replaced
      Given the app adopted a copy of the speech recognition service for commands
      When the adopted copy stops answering without ending
      Then within 30 seconds the app has stopped that copy's process and started its own copy on that port, and it answers
      And the log names the service, the copy, the port and the process id it stopped

    Scenario: Another program on the port is not killed
      Given the app adopted a copy of the speech recognition service for commands
      When another program that is not that service takes its port and does not answer
      Then after three failed checks the other program keeps running and the log names it with its process id

  Rule: A service that answers again before its restart is adopted

    Scenario: The service scripts restart a copy first
      Given the app adopted both copies of the speech recognition service for dictation
      When one adopted copy ends and the scripts start it again while the app's own start fails on the taken port
      Then the app adopts the copy the scripts started and no second copy runs on that port
      And the service is not Failed

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

  Rule: Another program on a speech recognition port is named in the log

    Scenario: A program on the dictation port is logged once
      Given another program that is no speech recognition service listens on the own port of the speech recognition service, which runs as one copy
      When a client of the service transcribes two dictations
      Then the log holds one warning naming that port, the program's name and its process id
