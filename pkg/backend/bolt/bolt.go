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
	BoardHandler
	ListHandler
	CardHandler
}

func NewWithDB(db *bolt.DB) (*Backend, error) {
	var err error
	b := new(Backend)
	b.BoardHandler, err = NewBoardHandler(db)
	if err != nil {
		return b, err
	}
	b.ListHandler, err = NewListHandler(db)
	if err != nil {
		return b, err
	}
	b.CardHandler, err = NewCardHandler(db)
	if err != nil {
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
