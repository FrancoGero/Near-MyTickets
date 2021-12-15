Nombre del Proyecto: WinWin ⚡

Proyecto desarrollado sobre Rust empleando los estandares NEP 171 y NEP 178. Su funcion es permitir la creación y venta de entradas para diferentes eventos en forma de NFT. Permite ademas funciones como la reventa de tickets con un fee que recibe el creador de las entradas. 
==================

Guía 📝
===========
Esta app fue creada con [create-near-app]
Para correr el proyecto en local debe realizar los siguientes pasos:

1. ✔️ Prerrequisito: Asegurese de tener instalado [Node.js] ≥ 12
2. ✔️ Instalar dependencias: `yarn install`
3. ✔️ Correr el servidor local de desarrollo: `yarn dev` (revisar el archivo `package.json` para conocer la lista completa de `scripts` que se pueden correr con `yarn`)

Explorando el Codido 🔎
======================

1. El codigo "backend" esta en la carpeta `/contract`. Las funcionalidades el proyecto se han cubierto con 4 funciones:

📌 Funcion: init
   Descripcion: Inicializar un contrato, pero si ya este se encuentra inicializado arroja un panic error
   Comando:    
    near call $ID init '{"id_admin": "'$ID2'", "metadata": { "spec": "prueba", "name": "token", "symbol": "MTT" }, "fee_reventa": { "num": 5,"den": 100}, "fee_reventa_id_address": "'$ID2'" }' --accountId $ID2}

📌 Funcion: crear_coleccion
   Descripcion: Crea una coleccion con la cantidad de tickects que se le indiquen
   Comando:    
    near call $ID crear_coleccion '{ "id_creador": "'$ID2'", "gate_id": "a1", "titulo": "titulo de ejemplo", "descripcion": "descripcion de ej", "cantidad": 10}' --accountId $ID2

📌 Funcion: get_coleccion_de_creador
   Descripcion: Obtiene todos los tickects creados por el usuario indicado, el resultado se muestra en forma de lista
   Comando:    
    near call $ID get_coleccion_de_creador '{"id_creador": "'$ID2'"}' --accountId $ID2

   Funcion: comprar_entrada
   Descripcion: Funcion para comprar un ticket
   Comando:    
    near call $ID comprar_entrada '{"gate_id": "a1"}' --accountId $ID2

2. Tests: Hemos desarrollado los siguientes 4 test sobre las funciones anteriores:

   📌cargo
   📌contexto
   📌inicializar_contrato
   📌crear_tickets
   📌comprar_tickets

    Para ejecutarlos nos posicionamos sobre la carpeta test y corremos el comando:

      cargo test -- --nocapture
   
Deployado 🚀
==============

Cada contrato en NEAR tiene su [tiene su propia cuenta asociada][NEAR accounts]. Cuando se corre el comando `yarn dev`, el smart contract se despliega en NEAR TestNet a traves de nuestra cuenta. Cuando se decida hacerlo permanente se deben seguir las siguientes instrucciones:


✔️ Paso 0: Instalar near-cli (opcional)
-------------------------------------

[near-cli] es una comando de interfaz (CLI) para interactuar con la NEAR blockchain. Se instala en la carpeta local `node_modules` cuando corremos el comando `yarn install`, pero para un mejor desempeño tambien se puede instalar globalmente:

    yarn install --global near-cli

O si prefiere la version local puede usar el prefijo `near` en los comandos `npx`

Asegurese de que se ha instalado con `near --version` (o `npx near --version`)

✔️ Paso 1: Crear una cuenta para el contrato
------------------------------------------

Cada cuenta NEAR puede tener al menos un contrato deployado. Si actualmente ha creado una cuenta `your-name.testnet`, puede deployar su contrato hacia `MyTickect.franco-geroli.testnet`. Asumiendo que ya ha creado una cuenta en [NEAR Wallet], estos serian los pasos para crear `MyTickect.franco-geroli.testnet`:

1. Autorizar NEAR CLI, con los siguientes comandos:

      near login

2. Crear una subcuenta (replaza `YOUR-NAME` con tu actual nombre de cuenta):

      near create-account WinWin.YOUR-NAME.testnet --masterAccount YOUR-NAME.testnet

✔️ Paso 2: Poner nombre del contrato en el codigo
---------------------------------

Modificar la linea en `src/config.js` que asigna nombre al contrato. 

    const CONTRACT_NAME = process.env.CONTRACT_NAME || 'WinWin.YOUR-NAME.testnet'


✔️ Paso 3: Deployado
---------------

Comando:

    yarn deploy

Solución de problemas
=======================

En Windows, si visualiza un error conteniendo `EPERM` puede estar relacionado con los espacios en el path. Por favor revisar [este tema](https://github.com/zkat/npx/issues/209) para mas detalle.


  [create-near-app]: https://github.com/near/create-near-app
  [Node.js]: https://nodejs.org/en/download/package-manager/
  [jest]: https://jestjs.io/
  [NEAR accounts]: https://docs.near.org/docs/concepts/account
  [NEAR Wallet]: https://wallet.testnet.near.org/
  [near-cli]: https://github.com/near/near-cli
  [gh-pages]: https://github.com/tschaub/gh-pages
