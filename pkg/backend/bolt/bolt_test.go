package bolt

import (
	"io/ioutil"
	"os"
	"path/filepath"
	"testing"

	"github.com/twistedogic/orga/pkg/testutil"
)

func Test_Backend(t *testing.T) {
	dir, err := ioutil.TempDir("", "bolt_backend")
	if err != nil {
		t.Fatal(err)
	}
	f := filepath.Join(dir, "test_db")
	defer os.RemoveAll(dir)
	b, err := New(f)
	if err != nil {
		t.Fatal(err)
	}
	testutil.TestBackend(t, b)
}
