"""Intentional jvozba input and enum type errors."""

from jbotci import jvozba

word = jvozba.Word("lojbo")
other = jvozba.Word("bangu")

jvozba.build("lojbo")
jvozba.build(["lojbo", "bangu"])
jvozba.build({word, other})
jvozba.build(iter((word, other)))
jvozba.build([word, other], mode="lujvo")
