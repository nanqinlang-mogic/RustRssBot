#!/bin/bash
Green_font="\033[32m" && Yellow_font="\033[33m" && Red_font="\033[31m" && Font_suffix="\033[0m"
Info="${Green_font}[Info]${Font_suffix}"
Error="${Red_font}[Error]${Font_suffix}"
echo -e "${Green_font}
#================================================================
# Project: RustRssBot
# Author:  nanqinlang
# Blog:    https://sometimesnaive.org
# Github:  https://github.com/nanqinlang
#================================================================
${Font_suffix}"

check_system(){
	[[ -z "`cat /etc/issue | grep -iE "debian"`" && -z "`cat /etc/issue | grep -iE "ubuntu"`" ]] && echo -e "${Error} only support Debian !" && exit 1
}

check_root(){
	[[ "`id -u`" != "0" ]] && echo -e "${Error} must be root user !" && exit 1
}

directory(){
	[[ ! -d /home/RustRssBot ]] && mkdir -p /home/RustRssBot
	cd /home/RustRssBot
}

download(){
	wget https://github.com/nanqinlang-mogic/RustRssBot/releases/download/1.0/rssbot
	[[ ! -f rssbot ]] && echo -e "${Error} download failed, please check !" && exit 1
}

define(){
	echo -e "${Info} input TELEGRAM-BOT-TOKEN of your bot :"
	read -p TELEGRAM-BOT-TOKEN
}

define-check(){
	while [[ -z ${TELEGRAM-BOT-TOKEN} ]]
	do
		echo -e "${Error} TOKEN could not be empty !" && define
	done
}

service(){
	echo -e "#!/bin/bash \ncd /home/RustRssBot \nnohup ./rssbot datafile ${TELEGRAM-BOT-TOKEN} &" >> rssbot-selfstart.sh && chmod +x rssbot-selfstart.sh
}

self-start(){
	sed -i "s/exit 0/ /ig" /etc/rc.local
	echo -e "\n/home/RustRssBot/rssbot-selfstart.sh\c" >> /etc/rc.local
	chmod +x /etc/rc.local
}

start(){
	./rssbot-selfstart.sh
}


# the main :
#check_system
check_root
directory
download
define
define-check
service
#self-start
start