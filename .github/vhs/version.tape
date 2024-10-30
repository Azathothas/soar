##Prints Version: soar --version
##SELF: 
#Setup ENV
Output version.gif
Require soar
Set Shell "bash"
Set WindowBar Colorful
Set Padding 0
#Set Theme "rose-pine"
Env PROMPT "➜ "
Env PS1 "➜ "
Sleep 500ms
#Actual CMD
Type@150ms "soar --version"
Enter
Sleep 2s