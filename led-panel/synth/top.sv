module top(
    input clk,
    input rst_n,
    output bit [7:0] led,
    output bit bs_10a, bs_10b, bs_10c, bs_10d, bs_10e, bs_10f, bs_10g,
    output bit bs_1a, bs_1b, bs_1c, bs_1d, bs_1e, bs_1f, bs_1g,
    output bit ws_10a, ws_10b, ws_10c, ws_10d, ws_10e, ws_10f, ws_10g,
    output bit ws_1a, ws_1b, ws_1c, ws_1d, ws_1e, ws_1f, ws_1g,
    output bit bto_15s, bto_30s, bto_45s, bto_60s, bto_int,
    output bit wto_15s, wto_30s, wto_45s, wto_60s, wto_int,
    output bit m_10a, m_10b, m_10c, m_10d, m_10e, m_10f, m_10g,
    output bit m_1a, m_1b, m_1c, m_1d, m_1e, m_1f, m_1g,
    output bit s_10a, s_10b, s_10c, s_10d, s_10e, s_10f, s_10g,
    output bit s_1a, s_1b, s_1c, s_1d, s_1e, s_1f, s_1g,
    output bit bto_ind, wto_ind, rto_ind,
    output bit fst_hlf, hlf_tm, snd_hlf, overtime, sdn_dth,
    output bit colon,
    input usb_rx,
    output bit usb_tx
    );
    
    wire rst;
    
    reset_conditioner reset_conditioner(.clk(clk), .in(!rst_n), .out(rst));

    logic timed_out = 1'b0;
    logic receiving = 1'b0;
    logic byte_rx = 1'b0;
    logic clrd = 1'b0;

    wire rx_complete_s, rx_complete_r, rx_complete;
    sr_ff rx_complete_ff(.clk(clk), .rst(rst), .s(rx_complete_s), .r(rx_complete_r), .q(rx_complete));

    wire disconnected_s, disconnected_r, disconnected;
    sr_ff disconnected_ff(.clk(clk), .rst(rst), .s(disconnected_s), .r(disconnected_r), .q(disconnected));

    wire data_ready;
    wire [7:0] data;
    UART_RX #(.CLKS_PER_BIT(868)) uart_rx(.i_Rst_L(!rst), .i_Clock(clk), .i_RX_Serial(usb_rx), .o_RX_DV(data_ready), .o_RX_Byte(data));

    localparam [4:0] 
        IDLE = 5'd0,
        DATA0 = 5'd1,
        DATA1 = 5'd2,
        DATA2 = 5'd3,
        DATA3 = 5'd4,
        DATA4 = 5'd5,
        DATA5 = 5'd6,
        DATA6 = 5'd7,
        DATA7 = 5'd8,
        DATA8 = 5'd9,
        DATA9 = 5'd10,
        DATA10 = 5'd11,
        DATA11 = 5'd12,
        DATA12 = 5'd13,
        DATA13 = 5'd14,
        DATA14 = 5'd15,
        DATA15 = 5'd16,
        DATA16 = 5'd17,
        DATA17 = 5'd18,
        DATA18 = 5'd19,
        DONE = 5'd20;

    reg [4:0] state = IDLE;
    logic [7:0] data_out [19:0];
    logic [7:0] data_q [19:0] = 160'h0;

    reg [15:0] timeout_counter = 16'b0;
    
    always @(posedge clk or posedge rst) begin
        if (rst) begin
            state <= IDLE;
            timeout_counter <= 16'b0;
            timed_out <= 1'b0;
            byte_rx <= 1'b0;
            receiving <= 1'b0;
            disconnected_r <= 1'b0;
            rx_complete_s <= 1'b0;
        end else begin
            if (disconnected) begin
                data_q <= 160'h0;
                disconnected_r <= 1'b1;
                clrd <= !clrd;
            end else begin
                disconnected_r <= 1'b0;
            end

            if (state == DONE) begin
                data_q <= data_out;
                rx_complete_s <= 1'b1;
                receiving <= 1'b0;
                state <= IDLE;
            end else begin
                rx_complete_s <= 1'b0;

                if (data_ready) begin
                    timeout_counter <= 16'b0;
                    byte_rx <= ~byte_rx;
                    case (state)
                        IDLE: begin
                            data_out[0] <= data;
                            timed_out <= 1'b0;
                            receiving <= 1'b1;
                            state <= DATA0;
                        end
                        DATA0: begin
                            data_out[1] <= data;
                            state <= DATA1;
                        end
                        DATA1: begin
                            data_out[2] <= data;
                            state <= DATA2;
                        end
                        DATA2: begin
                            data_out[3] <= data;
                            state <= DATA3;
                        end
                        DATA3: begin
                            data_out[4] <= data;
                            state <= DATA4;
                        end
                        DATA4: begin
                            data_out[5] <= data;
                            state <= DATA5;
                        end
                        DATA5: begin
                            data_out[6] <= data;
                            state <= DATA6;
                        end
                        DATA6: begin
                            data_out[7] <= data;
                            state <= DATA7;
                        end
                        DATA7: begin
                            data_out[8] <= data;
                            state <= DATA8;
                        end
                        DATA8: begin
                            data_out[9] <= data;
                            state <= DATA9;
                        end
                        DATA9: begin
                            data_out[10] <= data;
                            state <= DATA10;
                        end
                        DATA10: begin
                            data_out[11] <= data;
                            state <= DATA11;
                        end
                        DATA11: begin
                            data_out[12] <= data;
                            state <= DATA12;
                        end
                        DATA12: begin
                            data_out[13] <= data;
                            state <= DATA13;
                        end
                        DATA13: begin
                            data_out[14] <= data;
                            state <= DATA14;
                        end
                        DATA14: begin
                            data_out[15] <= data;
                            state <= DATA15;
                        end
                        DATA15: begin
                            data_out[16] <= data;
                            state <= DATA16;
                        end
                        DATA16: begin
                            data_out[17] <= data;
                            state <= DATA17;
                        end
                        DATA17: begin
                            data_out[18] <= data;
                            state <= DATA18;
                        end
                        DATA18: begin
                            data_out[19] <= data;
                            state <= DONE;
                        end
                    endcase
                end else if (timeout_counter >= 16'd34720) begin
                    receiving <= 1'b0;
                    state <= IDLE;
                    timed_out <= 1'b1;
                end else if (state != IDLE) begin
                    timeout_counter <= timeout_counter + 1;
                end
            end
        end
    end

    logic dcon = 1'b0;

    reg [25:0] disconenct_counter_d = 26'b0;
    reg [25:0] disconenct_counter_q = 26'b0;
    always @(posedge clk or posedge rst) begin
        if (rst) begin
            disconenct_counter_d <= 26'b0;
            disconenct_counter_q <= 26'b0;
            rx_complete_r <= 1'b0;
            disconnected_s <= 1'b0;
        end else begin
            disconenct_counter_q <= disconenct_counter_d;

            if (rx_complete) begin
                disconenct_counter_d <= 26'b0;
                rx_complete_r <= 1'b1;
            end else begin
                rx_complete_r <= 1'b0;

                if (disconenct_counter_q >= 26'd50000000) begin
                    // We haven't received any data in 0.5 seconds (we should have received 5
                    // messages), so we clear the display until a new message is received
                    disconnected_s <= 1'b1;
                    dcon <= !dcon;
                    disconenct_counter_d <= 26'b0;
                end else begin
                    disconnected_s <= 1'b0;
                    disconenct_counter_d <= disconenct_counter_q + 1;
                end
            end
        end
    end

    digit bs_10, bs_1, ws_10, ws_1, m_10, m_1, s_10, s_1;
    t_o_time bto, wto;
    wire bto_ind_en, wto_ind_en, rto_ind_en;
    wire fst_hlf_en, hlf_tm_en, snd_hlf_en, overtime_en, sdn_dth_en;
    wire colon_en;
    wire [1:0] brightness;
    segments segments(
        .data(data_q),
        .bs_10(bs_10), .bs_1(bs_1), .ws_10(ws_10), .ws_1(ws_1), .m_10(m_10), .m_1(m_1), .s_10(s_10), .s_1(s_1),
        .bto(bto), .wto(wto),
        .bto_ind(bto_ind_en), .wto_ind(wto_ind_en), .rto_ind(rto_ind_en),
        .fst_hlf(fst_hlf_en), .hlf_tm(hlf_tm_en), .snd_hlf(snd_hlf_en), .overtime(overtime_en), .sdn_dth(sdn_dth_en),
        .colon(colon_en),
        .brightness(brightness)
    );

    reg [10:0] pwm_counter = 11'b0;
    wire pwm_clk;

    assign pwm_clk = pwm_counter[10];

    always @(posedge clk or posedge rst) begin
        if (rst) begin
            pwm_counter <= 11'b0;
        end else begin
            pwm_counter <= pwm_counter + 1;
        end
    end

    wire [74:0] pwm_out;

    // Spacing 3
    reg [7:0] compare_low [75:0] = {8'hE1, 8'hDE, 8'hDB, 8'hD8, 8'hD5, 8'hD2, 8'hCF, 8'hCC, 8'hC9, 8'hC6, 8'hC3, 8'hC0, 8'hBD, 8'hBA, 8'hB7, 8'hB4, 8'hB1, 8'hAE, 8'hAB, 8'hA8, 8'hA5, 8'hA2, 8'h9F, 8'h9C, 8'h99, 8'h96, 8'h93, 8'h90, 8'h8D, 8'h8A, 8'h87, 8'h84, 8'h81, 8'h7E, 8'h7B, 8'h78, 8'h75, 8'h72, 8'h6F, 8'h6C, 8'h69, 8'h66, 8'h63, 8'h60, 8'h5D, 8'h5A, 8'h57, 8'h54, 8'h51, 8'h4E, 8'h4B, 8'h48, 8'h45, 8'h42, 8'h3F, 8'h3C, 8'h39, 8'h36, 8'h33, 8'h30, 8'h2D, 8'h2A, 8'h27, 8'h24, 8'h21, 8'h1E, 8'h1B, 8'h18, 8'h15, 8'h12, 8'h0F, 8'h0C, 8'h09, 8'h06, 8'h03, 8'h00};
    wire [74:0] pwm_int_low;
    pwm #(.CTR_LEN(8), .NUM_OUTPUTS(75)) pwm_low(.clk(pwm_clk), .rst(rst), .compare(compare_low), .pwm(pwm_int_low));

    // Spacing 10
    reg [7:0] compare_med [25:0] = {8'hFA, 8'hF0, 8'hE6, 8'hDC, 8'hD2, 8'hC8, 8'hBE, 8'hB4, 8'hAA, 8'hA0, 8'h96, 8'h8C, 8'h82, 8'h78, 8'h6E, 8'h64, 8'h5A, 8'h50, 8'h46, 8'h3C, 8'h32, 8'h28, 8'h1E, 8'h14, 8'h0A, 8'h00};
    wire [24:0] pwm_int_med;
    pwm #(.CTR_LEN(8), .NUM_OUTPUTS(25)) pwm_med(.clk(pwm_clk), .rst(rst), .compare(compare_med), .pwm(pwm_int_med));

    // Spacing 21
    reg [7:0] compare_high [12:0] = {8'hFC, 8'hE7, 8'hD2, 8'hBD, 8'hA8, 8'h93, 8'h7E, 8'h69, 8'h54, 8'h3F, 8'h2A, 8'h15, 8'h00};
    wire [11:0] pwm_int_high;
    pwm #(.CTR_LEN(8), .NUM_OUTPUTS(12)) pwm_high(.clk(pwm_clk), .rst(rst), .compare(compare_high), .pwm(pwm_int_high));

    // Spacing 36
    reg [7:0] compare_outdoor [7:0] = {8'hFC, 8'hD8, 8'hB4, 8'h90, 8'h6C, 8'h48, 8'h24, 8'h00};
    wire [6:0] pwm_int_outdoor;
    pwm #(.CTR_LEN(8), .NUM_OUTPUTS(7)) pwm_outdoor(.clk(pwm_clk), .rst(rst), .compare(compare_outdoor), .pwm(pwm_int_outdoor));

    assign pwm_out = (brightness == 2'b00) ? pwm_int_low :
                     (brightness == 2'b01) ? {pwm_int_med[24:0], pwm_int_med[24:0], pwm_int_med[24:0]} :
                     (brightness == 2'b10) ? {pwm_int_high[2:0], pwm_int_high[11:0], pwm_int_high[11:0], pwm_int_high[11:0], pwm_int_high[11:0], pwm_int_high[11:0], pwm_int_high[11:0]} :
                     {pwm_int_outdoor[4:0], pwm_int_outdoor[6:0], pwm_int_outdoor[6:0], pwm_int_outdoor[6:0], pwm_int_outdoor[6:0], pwm_int_outdoor[6:0], pwm_int_outdoor[6:0], pwm_int_outdoor[6:0], pwm_int_outdoor[6:0], pwm_int_outdoor[6:0], pwm_int_outdoor[6:0]};

    assign bs_10a = bs_10.a && pwm_out[0];
    assign bs_10b = bs_10.b && pwm_out[1];
    assign bs_10c = bs_10.c && pwm_out[2];
    assign bs_10d = bs_10.d && pwm_out[3];
    assign bs_10e = bs_10.e && pwm_out[4];
    assign bs_10f = bs_10.f && pwm_out[5];
    assign bs_10g = bs_10.g && pwm_out[6];

    assign bs_1a = bs_1.a && pwm_out[7];
    assign bs_1b = bs_1.b && pwm_out[8];
    assign bs_1c = bs_1.c && pwm_out[9];
    assign bs_1d = bs_1.d && pwm_out[10];
    assign bs_1e = bs_1.e && pwm_out[11];
    assign bs_1f = bs_1.f && pwm_out[12];
    assign bs_1g = bs_1.g && pwm_out[13];

    assign ws_10a = ws_10.a && pwm_out[14];
    assign ws_10b = ws_10.b && pwm_out[15];
    assign ws_10c = ws_10.c && pwm_out[16];
    assign ws_10d = ws_10.d && pwm_out[17];
    assign ws_10e = ws_10.e && pwm_out[18];
    assign ws_10f = ws_10.f && pwm_out[19];
    assign ws_10g = ws_10.g && pwm_out[20];

    assign ws_1a = ws_1.a && pwm_out[21];
    assign ws_1b = ws_1.b && pwm_out[22];
    assign ws_1c = ws_1.c && pwm_out[23];
    assign ws_1d = ws_1.d && pwm_out[24];
    assign ws_1e = ws_1.e && pwm_out[25];
    assign ws_1f = ws_1.f && pwm_out[26];
    assign ws_1g = ws_1.g && pwm_out[27];

    assign bto_15s = bto.fifteen && pwm_out[28];
    assign bto_30s = bto.thirty && pwm_out[29];
    assign bto_45s = bto.forty_five && pwm_out[30];
    assign bto_60s = bto.sixty && pwm_out[31];
    assign bto_int = bto.interstice && pwm_out[32];

    assign wto_15s = wto.fifteen && pwm_out[33];
    assign wto_30s = wto.thirty && pwm_out[34];
    assign wto_45s = wto.forty_five && pwm_out[35];
    assign wto_60s = wto.sixty && pwm_out[36];
    assign wto_int = wto.interstice && pwm_out[37];

    assign m_10a = m_10.a && pwm_out[38];
    assign m_10b = m_10.b && pwm_out[39];
    assign m_10c = m_10.c && pwm_out[40];
    assign m_10d = m_10.d && pwm_out[41];
    assign m_10e = m_10.e && pwm_out[42];
    assign m_10f = m_10.f && pwm_out[43];
    assign m_10g = m_10.g && pwm_out[44];

    assign m_1a = m_1.a && pwm_out[45];
    assign m_1b = m_1.b && pwm_out[46];
    assign m_1c = m_1.c && pwm_out[47];
    assign m_1d = m_1.d && pwm_out[48];
    assign m_1e = m_1.e && pwm_out[49];
    assign m_1f = m_1.f && pwm_out[50];
    assign m_1g = m_1.g && pwm_out[51];

    assign s_10a = s_10.a && pwm_out[52];
    assign s_10b = s_10.b && pwm_out[53];
    assign s_10c = s_10.c && pwm_out[54];
    assign s_10d = s_10.d && pwm_out[55];
    assign s_10e = s_10.e && pwm_out[56];
    assign s_10f = s_10.f && pwm_out[57];
    assign s_10g = s_10.g && pwm_out[58];

    assign s_1a = s_1.a && pwm_out[59];
    assign s_1b = s_1.b && pwm_out[60];
    assign s_1c = s_1.c && pwm_out[61];
    assign s_1d = s_1.d && pwm_out[62];
    assign s_1e = s_1.e && pwm_out[63];
    assign s_1f = s_1.f && pwm_out[64];
    assign s_1g = s_1.g && pwm_out[65];

    assign bto_ind = bto_ind_en && pwm_out[66];
    assign wto_ind = wto_ind_en && pwm_out[67];
    assign rto_ind = rto_ind_en && pwm_out[68];

    assign fst_hlf = fst_hlf_en && pwm_out[69];
    assign hlf_tm = hlf_tm_en && pwm_out[70];
    assign snd_hlf = snd_hlf_en && pwm_out[71];
    assign overtime = overtime_en && pwm_out[72];
    assign sdn_dth = sdn_dth_en && pwm_out[73];

    assign colon = colon_en && pwm_out[74];

    assign led[0] = rx_complete;
    assign led[1] = usb_rx;
    assign led[2] = pwm_clk;
    assign led[3] = pwm_int_low[0];
    assign led[4] = disconenct_counter_q[25];
    assign led[5] = disconenct_counter_q[0];
    assign led[7:6] = brightness;

endmodule
