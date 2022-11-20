export function waste_time() {
    return new Promise((resolve,reject) => {
        setTimeout(()=> {
            resolve()
        }, 3000);
    });
}
