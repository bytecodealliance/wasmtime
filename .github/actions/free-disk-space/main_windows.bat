:: Using https://github.com/actions/virtual-environments/blob/main/images/win/Windows2022-Readme.md
:: Remove databases
rmdir /s /q C:\PROGRA~1\POSTGR~1
net stop MongoDB
rmdir /s /q C:\PROGRA~1\MongoDB
:: Remove browsers
rmdir /s /q C:\PROGRA~1\Google\Chrome
rmdir /s /q C:\PROGRA~1\MOZILL~1
:: Remove other
net stop docker
rmdir /s /q C:\PROGRA~1\Docker