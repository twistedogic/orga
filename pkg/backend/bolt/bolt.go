package bolt

import (
	bolt "go.etcd.io/bbolt"
)

const (
	boardBucketName = "board"
	listBucketName  = "list"
	cardBucketName  = "card"
)

type Backend struct {
	BoardHandler, ListHandler, CardHandler Store
}

func NewWithDB(db *bolt.DB) (*Backend, error) {
	b := new(Backend)
	b.BoardHandler = NewStore(boardBucketName, db)
	if err := b.BoardHandler.Init(); err != nil {
		return b, err
	}
	b.ListHandler = NewStore(listBucketName, db)
	if err := b.ListHandler.Init(); err != nil {
		return b, err
	}
	b.CardHandler = NewStore(cardBucketName, db)
	if err := b.CardHandler.Init(); err != nil {
		return b, err
	}
	return b, nil
}

func New(path string) (*Backend, error) {
	db, err := bolt.Open(path, 0600, nil)
	if err != nil {
		return nil, err
	}
	return NewWithDB(db)
}
