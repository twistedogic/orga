package config

import (
	"io/ioutil"
	"os"
	"path/filepath"
	"reflect"
	"testing"
)

func Test_Config(t *testing.T) {
	cases := map[string]Config{
		"base":       {Key: "a", Token: "b"},
		"incomplete": {Token: "b"},
	}
	for name := range cases {
		want := cases[name]
		t.Run(name, func(t *testing.T) {
			dir, err := ioutil.TempDir("", "config_test")
			if err != nil {
				t.Fatal(err)
			}
			defer os.RemoveAll(dir)
			filename := filepath.Join(dir, name)
			if err := Write(want, filename); err != nil {
				t.Fatal(err)
			}
			got, err := Read(filename)
			if err != nil {
				t.Fatal(err)
			}
			if !reflect.DeepEqual(want, got) {
				t.Fatalf("want: %#v, got: %#v", want, got)
			}
		})
	}
}
