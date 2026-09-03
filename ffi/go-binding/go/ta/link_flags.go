package ta

/*
#cgo linux LDFLAGS: -L${SRCDIR}/../../../../target/release -lfinkit_go -lm -ldl -lpthread
#cgo darwin LDFLAGS: -L${SRCDIR}/../../../../target/release -lfinkit_go
#cgo windows LDFLAGS: -L${SRCDIR}/../../../../target/release -lfinkit_go -lws2_32 -ladvapi32 -luserenv -lbcrypt -lncrypt -lschannel -luser32
*/
import "C"
