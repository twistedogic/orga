package config

import (
	"encoding/json"
	"fmt"
	"io/ioutil"
	"os/user"
	"path/filepath"

	"github.com/pkg/errors"
)

const (
	defaultFileName = ".orga.json"
)

func ConfigPath() (string, error) {
	u, err := user.Current()
	if err != nil {
		return "", err
	}
	if u.HomeDir == "" {
		return "", fmt.Errorf("no home directory found for %s", u.Name)
	}
	return filepath.Join(u.HomeDir, defaultFileName), nil
}

type Config struct {
	Key   string `json:"key,omitempty"`
	Token string `json:"token,omitempty"`
}

func (c Config) IsSet() bool {
	return c.Key != "" && c.Token != ""
}

func New(key, token string) Config {
	return Config{
		Key:   key,
		Token: token,
	}
}

func Read(path string) (Config, error) {
	var c Config
	b, err := ioutil.ReadFile(path)
	if err != nil {
		return c, err
	}
	err = json.Unmarshal(b, &c)
	return c, err
}

func Write(c Config, path string) error {
	b, err := json.Marshal(&c)
	if err != nil {
		return err
	}
	return ioutil.WriteFile(path, b, 0755)
}

func ReadConfig() (Config, error) {
	filename, err := ConfigPath()
	if err != nil {
		return Config{}, errors.Wrap(err, "get config file path")
	}
	return Read(filename)
}

func StoreConfig(c Config) error {
	filename, err := ConfigPath()
	if err != nil {
		return errors.Wrap(err, "get config file path")
	}
	return Write(c, filename)
}
