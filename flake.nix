{
	inputs = {
		nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
		nix-appimage.url = "github:ralismark/nix-appimage";
	};
	outputs = { self, nixpkgs, nix-appimage }: let
		system = "x86_64-linux";
		pkgs = nixpkgs.legacyPackages.${system};
	in {
		devShells.${system}.default = pkgs.mkShell {
			buildInputs = with pkgs; [ SDL2 luajit udev pkg-config cargo rustc ];
			shellHook = ''
				alias build_release='nix bundle --bundler . .#moonglide'
			'';
		};

		packages.${system} = let
			moonglide = pkgs.rustPlatform.buildRustPackage {
				pname = "moonglide";
				version = "git";
				src = let lib = pkgs.lib; in lib.cleanSourceWith {
					src = lib.cleanSource ./.;
					filter = path: _type:
						!(baseNameOf path == "target" || baseNameOf path == "result");
				};
				cargoLock.lockFile = ./Cargo.lock;
				nativeBuildInputs = [ pkgs.pkg-config ];
				meta.mainProgram = "moonglide";
				# Strip everything moonglide never touches (GL, Wayland/X11, all audio
				# backends incl. pipewire, drm, vulkan, ibus): joystick + sensor +
				# udev is all we use. Keeps the bundled AppImage closure small.
				buildInputs = let
					sdl3Lean = pkgs.sdl3.override {
						openglSupport = false;
						waylandSupport = false;
						x11Support = false;
						alsaSupport = false;
						pulseaudioSupport = false;
						pipewireSupport = false;
						jackSupport = false;
						drmSupport = false;
						ibusSupport = false;
						libdecorSupport = false;
						vulkanSupport = false;
						traySupport = false;
						dbusSupport = false;
					};
				in [
					(pkgs.SDL2.override { sdl3 = sdl3Lean; })
					pkgs.udev
					pkgs.luajit		# mlua 'luajit' feature links system LuaJIT
				];
			};
		in {
			inherit moonglide;
			default = moonglide;
		};

		apps.${system}.default = {
			type = "app";
			program = "${self.packages.${system}.default}/bin/moonglide";
		};

		bundlers.${system}.default = nix-appimage.bundlers.${system}.default;
	};
}
