##Prints List: soar list
##SELF: https://raw.githubusercontent.com/Azathothas/soar/refs/heads/main/.github/vhs/list.tape
#Setup ENV
Output list.gif
Require soar
Set Shell "bash"
Set WindowBar Colorful
Set Padding 0
Set PlaybackSpeed 0.4
#Set Theme "rose-pine"
Env PROMPT "➜ "
Env PS1 "➜ "
Sleep 500ms
#Actual CMD
Type@150ms "soar list"
Enter
Sleep 1s
Type@300ms "soar list | wc -l"
Enter
Sleep 2s
