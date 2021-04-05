package configure

import (
	"fmt"
	"os/exec"
	"runtime"
)

const (
	tokenURL = "https://trello.com/1/authorize?expiration=never&scope=read,write,account&response_type=token&name=%s&key=%s"
	appName  = "Orga"
)

func getTokenURL(key string) string {
	return fmt.Sprintf(tokenURL, appName, key)
}

func openBrowser(url string) error {
	var args []string
	switch runtime.GOOS {
	case "darwin":
		args = []string{"open"}
	case "windows":
		args = []string{"cmd", "/c", "start"}
	default:
		args = []string{"xdg-open"}

	}
	return exec.Command(args[0], append(args[1:], url)...).Start()
}
