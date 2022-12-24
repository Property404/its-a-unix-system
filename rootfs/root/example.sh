# This is a comment. You won't see it

echo "Would you like to see your fortune?"
# `read` read user input to a new variable `answer`
read -p '(y/n) > ' answer
# `[` evaluates a conditional. In this case, we're checking if
# the variable `answer` is "y".
[ "${answer}" =~ "y" ] || echo -e "Fine. \u0001f621"
# The `-s` flag selects a short fortune.
[ "${answer}" =~ "y" ] && echo -e "Here is a \u0001f42e with your fortune" && fortune -s | cowsay
