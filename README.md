# ClumsyLoader
ClumsyLoader is a single binary written in go that has the ability to download a specific backup of one of your servers on Pterodactyl Panel using the Pterodactyl Panel API.

An example use case for this tool is if you want to take your own backups, you can run this tool on a server of some kind and have it run on a schedule, constantly downloading your backups for you automatically :).

I'm a very novice coder so there isn't any checks currently to see if a user messed something up. Please just make sure to fill in the .env file with your information :)

EXAMPLE .env
```
SERVERID=d4e625e1
APIKEY=f1753fgsqNy4IFuso2534wsdggMOSK1235sg0Rb
BACKUPNUM=0
PANELURL=mc.bloom.host
```

Please keep in mind BACKUPNUM is in array, so start at 0 and count up (0 being your oldest backup).

With Pterodactyl's automatic clearing of the oldest backup when ran by a schedule, you can use this tool to automatically download the oldest backup every week, day, etc.
