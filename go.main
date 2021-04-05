

package main

import (
	"fmt"
	"github.com/buger/jsonparser"
	"github.com/cavaliercoder/grab"
	"github.com/fatih/color"
	"github.com/joho/godotenv"
	"io/ioutil"
	"log"
	"net/http"
	"os"
	"time"
)

const second = time.Second

func main() {


	err := godotenv.Load()
	if err != nil {
		log.Fatal("Error loading .env file")
		color.Red("Error loading .env file")
	}

	serverID := os.Getenv("SERVERID")
	apiKEY := os.Getenv("APIKEY")
	backupNUM := os.Getenv("BACKUPNUM")
	panelURL := os.Getenv("PANELURL")


	req, err := http.NewRequest("GET", "https://" + panelURL + "/api/client/servers/" +serverID + "/backups", nil)
	if err != nil {
		// handle err
	}
	req.Header.Set("Accept", "application/json")
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("Authorization", "Bearer " + apiKEY)

	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		// handle err
	}
	defer resp.Body.Close()

	body, _ := ioutil.ReadAll(resp.Body)

	uuid, _ := jsonparser.GetString(body, "data", "[" + backupNUM + "]", "attributes", "uuid")
	//fmt.Println(uuid)

	req1, err1 := http.NewRequest("GET", "https://" + panelURL + "/api/client/servers/" + serverID + "/backups/" + uuid + "/download", nil)
	if err1 != nil {
		// handle err
	}
	req1.Header.Set("Accept", "application/json")
	req1.Header.Set("Content-Type", "application/json")
	req1.Header.Set("Authorization", "Bearer " + apiKEY)


	resp1, err1 := http.DefaultClient.Do(req1)
	if err1 != nil {
		// handle err
	}
	defer resp1.Body.Close()

	body1, _ := ioutil.ReadAll(resp1.Body)
	dlLink, _ := jsonparser.GetString(body1, "attributes", "url")


	////////////////////////////////////////////

	// create client
	client := grab.NewClient()
	req2, _ := grab.NewRequest(".", dlLink)

	// color defining
	blueText := color.New(color.FgCyan, color.Bold)
	greenText := color.New(color.FgHiGreen, color.Bold)

	// start download
	blueText.Printf("Downloading %v...\n", req2.URL())
	resp2 := client.Do(req2)
	greenText.Printf("  %v\n", resp2.HTTPResponse.Status)

	// start UI loop
	t := time.NewTicker(5000 * time.Millisecond)
	defer t.Stop()

Loop:
	for {
		select {
		case <-t.C:
			greenText.Printf("  transferred %v / %v megabytes (%.2f%%)\n",
				resp2.BytesComplete()/1048576,
				resp2.Size()/1048576,
				100*resp2.Progress())

		case <-resp2.Done:
			// download is complete
			break Loop
		}
	}

	// check for errors
	if err := resp2.Err(); err != nil {
		fmt.Fprintf(os.Stderr, "Download failed: %v\n", err)
		os.Exit(1)
	}

	fmt.Printf("Download saved to ./%v \n", resp2.Filename)


}
