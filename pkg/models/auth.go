package models

type RegisterForm struct {
	Username string `json:"username"`
	Password string `json:"password"`
	Email    string `json:"email"`
}

type LoginForm struct {
	Username string `json:"username"`
	Password string `json:"password"`
}
