package configure

import (
	"fmt"
	"strings"

	"github.com/urfave/cli/v2"

	"github.com/twistedogic/orga/pkg/config"
)

var (
	keyVar, tokenVar string
	configureFlags   = []cli.Flag{
		&cli.StringFlag{
			Name:        "key",
			Aliases:     []string{"k"},
			EnvVars:     []string{"TRELLO_APP_KEY"},
			Usage:       "trello app key",
			Destination: &keyVar,
		},
		&cli.StringFlag{
			Name:        "token",
			Aliases:     []string{"t"},
			EnvVars:     []string{"TRELLO_API_TOKEN"},
			Usage:       "trello api token",
			Destination: &tokenVar,
		},
	}
)

func Input(format, value string) string {
	var out string
	fmt.Printf(format, value)
	fmt.Scanln(&out)
	return strings.TrimSpace(out)
}

func Prompt(c *config.Config) {
	key := Input("Please input trello app key (current:\"%s\"):\n> ", c.Key)
	if key != "" {
		c.Key = key
	}
	u := getTokenURL(c.Key)
	fmt.Printf("\nPlease go to:\n\n%s\n\n", u)
	token := Input("Please input trello api token (current:\"%s\"):\n> ", c.Token)
	if token != "" {
		c.Token = token
	}
}

func Run(ctx *cli.Context) error {
	c, err := config.ReadConfig()
	if err != nil {
		c = config.Config{}
	}
	Prompt(&c)
	return config.StoreConfig(c)
}

func Command() *cli.Command {
	return &cli.Command{
		Name:   "configure",
		Usage:  "configure orga config",
		Action: Run,
	}
}
