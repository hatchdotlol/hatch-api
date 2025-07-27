package emails

import (
	"bytes"
	"embed"
	"fmt"
	htmlTmpl "html/template"
	txtTmpl "text/template"

	"github.com/hatchdotlol/hatch-api/pkg/util"
	gomail "gopkg.in/mail.v2"
)

var emailSubjects = map[string]string{
	"verify": "Verify your email address",
}

type EmailTmplVars struct {
	PlatformName     string
	PlatformLogo     string
	PlatformFrontend string

	FromName    string
	FromAddress string

	Subject   string
	ToName    string
	ToAddress string
	Token     string
}

//go:embed templates/*
var templates embed.FS

func SendEmail(tmplName, toName, toAddress, token string) error {
	vars := EmailTmplVars{
		PlatformName:     util.Config.Mail.PlatformName,
		PlatformLogo:     util.Config.Mail.PlatformLogo,
		PlatformFrontend: util.Config.Mail.PlatformFrontend,

		FromName:    util.Config.Mail.FromName,
		FromAddress: util.Config.Mail.FromAddress,

		Subject:   emailSubjects[tmplName],
		ToName:    toName,
		ToAddress: toAddress,
		Token:     token,
	}

	m := gomail.NewMessage()
	m.SetHeader("From", fmt.Sprintf("%s <%s>", vars.FromName, vars.FromAddress))
	m.SetHeader("To", fmt.Sprintf("%s <%s>", vars.ToName, vars.ToAddress))
	m.SetHeader("Subject", vars.Subject)

	var txtTmplBuf, htmlTmplBuf bytes.Buffer
	_, err := txtTmpl.ParseFS(templates, "templates/base.txt", fmt.Sprintf("templates/%s.txt", tmplName))
	if err != nil {
		return err
	}

	ht, err := htmlTmpl.ParseFS(templates, "templates/base.html", fmt.Sprintf("templates/%s.html", tmplName))
	if err != nil {
		return err
	}
	ht.Execute(&htmlTmplBuf, &vars)

	m.SetBody("text/plain", txtTmplBuf.String())
	m.AddAlternative("text/html", htmlTmplBuf.String())

	if err := gomail.NewDialer(
		util.Config.Mail.EmailSMTPHost,
		util.Config.Mail.EmailSMTPPort,
		util.Config.Mail.EmailSMTPUsername,
		util.Config.Mail.EmailSMTPPassword,
	).DialAndSend(m); err != nil {
		return err
	}

	return nil
}
