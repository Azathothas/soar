##Prints List: soar list
##SELF: https://raw.githubusercontent.com/Azathothas/soar/refs/heads/main/.github/vhs/list.tape
#Setup ENV
Output list.gif
Require soar
Set Shell "bash"
Set WindowBar Colorful
Set Padding 0
#Set PlaybackSpeed 0.8
#Set Theme "rose-pine"
Env PROMPT "➜ "
Env PS1 "➜ "
Sleep 500ms
#Actual CMD
Type@800ms "soar list base"
Enter
Sleep 2s
Type@800ms "soar list bin"
Enter
Sleep 2s
Type@800ms "soar list pkg"
Enter
Sleep 2s
Type@450ms "soar list | wc -l"
Enter
Sleep 2s
