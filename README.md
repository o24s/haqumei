NOTE:  
# Analysis Open JTalk
This repository is a fork of '***fork of open_jtalk***' ([`r9y9/open_jtalk`](https://github.com/r9y9/open_jtalk)).  
I added only NOTE comments for analysis/understanding of Open JTalk.  
The comments start with `NOTE:` notation.  

NOTE: There are some differences between Open JTalk cvs and `r9y9/open_jtalk`.  
Open JTalk cvs stop update at v1.10, but there is the Open JTalk v1.11 as archive file in official Open JTalk page.  
`r9y9/open_jtalk` patch v1.11 into v1.10, prepare CIs, and fix bugs for Open JTalk users (especially for pyopenjtalk user).

# open_jtalk

[![C/C++ CI](https://github.com/r9y9/open_jtalk/actions/workflows/ccpp.yaml/badge.svg)](https://github.com/r9y9/open_jtalk/actions/workflows/ccpp.yaml)

A fork of open_jtalk based on v1.10.

## Why

Wanted to fork it with *git*.

**NOTE**: To preserve history of cvs version of open_jtalk, this fork was originially created by:

```
git cvsimport -v \
  -d :pserver:anonymous@open-jtalk.cvs.sourceforge.net:/cvsroot/open-jtalk \
  -C open_jtalk open_jtalk
```
