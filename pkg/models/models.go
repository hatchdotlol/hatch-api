package models

type Register struct {
	Username string `json:"username"`
	Password string `json:"password"`
	Email    string `json:"email"`
}

type Login struct {
	Username string `json:"username"`
	Password string `json:"password"`
}

type UserInfo struct {
	Bio     string
	Country string
}

type AddComment struct {
	Content string `json:"content"`
	ReplyTo *int64 `json:"replyTo"`
}
