# Spec Delta

## ADDED Requirements

### Requirement: Every event of audio accepted before a sync is out before the sync is answered
When an app asks the command spotting service to confirm that every event of the audio it sent so far is out (a sync), the service SHALL answer only after it has reported every speech segment closed in that audio, also a segment whose confirmation it had to ask again — because the first hearing came back empty, cut short under load or misheard — after the sync arrived; an app that closes the connection on that answer SHALL have received every command said in that audio.

#### Scenario: A command whose first hearing is empty under load is reported before the sync is answered
- **GIVEN** voice control runs in standby and the speech recognition service for commands answers the first hearing of a segment empty and late, as under machine load, and the next one right
- **WHEN** the user says "clear text please" alone and the app asks for a sync right after the speech
- **THEN** the command spotting service reports "clear all text" before it answers the sync

#### Scenario: A command whose half-pause hearing is misheard is reported before the sync is answered
- **GIVEN** voice control runs in standby and the speech recognition service for commands mishears a command word in the hearing asked at half the pause and hears it right in the hearing of the whole segment
- **WHEN** the user says "okay, clear" with a short pause after "okay" and the app asks for a sync right after the speech
- **THEN** the command spotting service reports "clear all text" before it answers the sync
