package api

import (
	"log"
	"net/http"
	"sync"

	"github.com/go-chi/chi/v5"
	"github.com/gorilla/websocket"
	"github.com/hatchdotlol/hatch-api/pkg/users"
)

func NotificationRouter() *chi.Mux {
	r := chi.NewRouter()

	r.Group(func(r chi.Router) {
		r.Use(EnsureVerifiedWs)
		r.Get("/", root)
	})

	return r
}

var upgrader = websocket.Upgrader{
	ReadBufferSize:  1024,
	WriteBufferSize: 1024,
	CheckOrigin: func(r *http.Request) bool {
		return true
	},
}

type Connections struct {
	lock  sync.RWMutex
	conns map[users.User]*websocket.Conn
}

func (c *Connections) Register(user users.User, conn *websocket.Conn) {
	c.lock.Lock()
	defer c.lock.Unlock()
	c.conns[user] = conn
}

func (c *Connections) Unregister(user users.User) {
	c.lock.Lock()
	defer c.lock.Unlock()
	delete(c.conns, user)
}

func (c *Connections) Broadcast(filter func(users.User) bool, message []byte) {
	c.lock.RLock()
	defer c.lock.RUnlock()
	for user, conn := range c.conns {
		if filter(user) {
			conn.WriteMessage(websocket.TextMessage, message)
		}
	}
}

var connections = Connections{
	conns: make(map[users.User]*websocket.Conn),
}

func root(w http.ResponseWriter, r *http.Request) {
	conn, err := upgrader.Upgrade(w, r, nil)
	if err != nil {
		log.Println(err)
		return
	}
	defer conn.Close()

	user := r.Context().Value(User).(users.User)

	connections.Register(user, conn)
	defer connections.Unregister(user)

	for {
		messageType, _, err := conn.ReadMessage()
		if err != nil || messageType == websocket.CloseMessage {
			break
		}
	}
}
