package bolt

import (
	"encoding/json"
	"fmt"

	bolt "go.etcd.io/bbolt"
)

type Store struct {
	name []byte
	*bolt.DB
}

func NewStore(name string, db *bolt.DB) Store {
	return Store{
		name: []byte(name),
		DB:   db,
	}
}

func (s Store) Init() error {
	return s.Update(func(tx *bolt.Tx) error {
		_, err := tx.CreateBucketIfNotExists(s.name)
		return err
	})
}

func (s Store) Get(key string, i interface{}) error {
	return s.View(func(tx *bolt.Tx) error {
		if b := tx.Bucket(s.name).Get([]byte(key)); b != nil {
			return json.Unmarshal(b, i)
		}
		return fmt.Errorf("not key %s found", key)
	})
}

func (s Store) Set(key string, i interface{}) error {
	return s.Update(func(tx *bolt.Tx) error {
		b, err := json.Marshal(i)
		if err != nil {
			return err
		}
		return tx.Bucket(s.name).Put([]byte(key), b)
	})
}

func (s Store) Delete(key string) error {
	return s.Update(func(tx *bolt.Tx) error {
		return tx.Bucket(s.name).Delete([]byte(key))
	})
}

func (s Store) List() ([]string, error) {
	out := make([]string, 0)
	err := s.View(func(tx *bolt.Tx) error {
		return tx.Bucket(s.name).ForEach(func(k, _ []byte) error {
			out = append(out, string(k))
			return nil
		})
	})
	return out, err
}
